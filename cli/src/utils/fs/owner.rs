#[cfg(unix)]
use crate::utils::os::unix::fs::owner as imp;
#[cfg(windows)]
use crate::utils::os::windows::fs::owner as imp;
use std::io;

#[cfg(not(any(windows, unix)))]
mod imp {
    use super::*;

    pub(crate) struct User;
    impl User {
        pub(crate) fn from_name(_: &str) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by name",
            ))
        }
    }
    pub(crate) struct Group;
    impl Group {
        pub(crate) fn from_name(_: &str) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by name",
            ))
        }
    }
}

enum UserInner {
    Resolved(imp::User),
    #[cfg(unix)]
    Numeric(u64),
}

pub(crate) struct User(UserInner);

impl User {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        imp::User::from_name(name).map(|user| Self(UserInner::Resolved(user)))
    }

    #[inline]
    #[allow(unused_variables)]
    pub(crate) fn from_uid(uid: u64) -> io::Result<Self> {
        #[cfg(unix)]
        {
            match imp::User::from_uid((uid as u32).into()) {
                Ok(user) => Ok(Self(UserInner::Resolved(user))),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    Ok(Self(UserInner::Numeric(uid)))
                }
                Err(e) => Err(e),
            }
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by uid",
            ))
        }
    }

    #[inline]
    pub(crate) fn name(&self) -> Option<&str> {
        match &self.0 {
            UserInner::Resolved(user) => {
                #[cfg(any(unix, windows))]
                {
                    Some(user.name())
                }
                #[cfg(not(any(unix, windows)))]
                {
                    let _ = user;
                    None
                }
            }
            #[cfg(unix)]
            UserInner::Numeric(_) => None,
        }
    }

    #[inline]
    pub(crate) fn uid(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            match &self.0 {
                UserInner::Resolved(user) => Some(user.as_raw() as _),
                UserInner::Numeric(uid) => Some(*uid),
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    #[inline]
    pub(crate) fn primary_gid(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            match &self.0 {
                UserInner::Resolved(user) => user.primary_gid().map(|gid| gid as _),
                UserInner::Numeric(_) => None,
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }
}

enum GroupInner {
    Resolved(imp::Group),
    #[cfg(unix)]
    Numeric(u64),
}

pub(crate) struct Group(GroupInner);

impl Group {
    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        imp::Group::from_name(name).map(|group| Self(GroupInner::Resolved(group)))
    }

    #[inline]
    #[allow(unused_variables)]
    pub(crate) fn from_gid(gid: u64) -> io::Result<Self> {
        #[cfg(unix)]
        {
            match imp::Group::from_gid((gid as u32).into()) {
                Ok(group) => Ok(Self(GroupInner::Resolved(group))),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    Ok(Self(GroupInner::Numeric(gid)))
                }
                Err(e) => Err(e),
            }
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "can not find by gid",
            ))
        }
    }

    #[inline]
    pub(crate) fn name(&self) -> Option<&str> {
        match &self.0 {
            GroupInner::Resolved(group) => {
                #[cfg(any(unix, windows))]
                {
                    Some(group.name())
                }
                #[cfg(not(any(unix, windows)))]
                {
                    let _ = group;
                    None
                }
            }
            #[cfg(unix)]
            GroupInner::Numeric(_) => None,
        }
    }

    #[inline]
    pub(crate) fn gid(&self) -> Option<u64> {
        #[cfg(unix)]
        {
            match &self.0 {
                GroupInner::Resolved(group) => Some(group.as_raw() as _),
                GroupInner::Numeric(gid) => Some(*gid),
            }
        }
        #[cfg(not(unix))]
        {
            None
        }
    }
}
