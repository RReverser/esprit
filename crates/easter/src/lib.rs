extern crate joker;
extern crate tristate;
extern crate serde;

#[cfg(feature = "nightly")]
#[macro_use]
extern crate derive;

#[cfg(feature = "nightly")]
macro_rules! pub_mod {
    ($name:ident) => (pub mod $name;)
}

#[cfg(not(feature = "nightly"))]
macro_rules! pub_mod {
    ($name:ident) => (pub mod $name {
        include!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".rs"));
    })
}

macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

macro_rules! serialize {
    ($name:ident as $implementation:block) => {
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, out: S) -> Result<S::Ok, S::Error>
                $implementation
        }
    }
}

macro_rules! json {
    (($key:ident : $value:expr),*) => {
        let mut map = out.serialize_map(count!($($key)*))?;
        $(
            map.serialize_key(stringify!($key))?;
            map.serialize_value($value)?;
        )*
        map.end()
    }
}

pub_mod!(id);
pub_mod!(fun);
pub_mod!(obj);
pub_mod!(stmt);
pub_mod!(expr);
pub_mod!(decl);
pub_mod!(patt);
pub_mod!(punc);
pub_mod!(cover);
