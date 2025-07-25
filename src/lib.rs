#![allow(
    clippy::needless_late_init,
    clippy::comparison_chain,
    clippy::question_mark,
    clippy::type_complexity
)]

extern crate ahash;
extern crate ansi_term;
extern crate atty;
extern crate chrono;
extern crate chrono_tz;
extern crate chronoutil;
extern crate dirs;
#[cfg(feature = "fxhash")]
extern crate fxhash;
extern crate iana_time_zone;
extern crate indexmap;
extern crate ipnet;
extern crate ipnetwork;
extern crate iprange;
extern crate lazy_static;
extern crate memchr;
extern crate md5;
extern crate netstat2;
extern crate nix;
extern crate nonblock;
extern crate num;
extern crate num_bigint;
extern crate num_traits;
extern crate pnet;
extern crate rand;
extern crate regex;
extern crate roxmltree;
extern crate rust_decimal;
extern crate rustyline;
extern crate rustyline_derive;
extern crate searchpath;
extern crate serde;
extern crate sha1;
extern crate sha2;
extern crate sqlx;
extern crate sysinfo;
extern crate tempfile;
extern crate term_size;
extern crate termion;
extern crate unicode_segmentation;
extern crate utime;
extern crate xml;

#[macro_use]
pub mod chunk;
pub mod compiler;
pub mod hasher;
mod opcode;
pub mod rl;
pub mod vm;
