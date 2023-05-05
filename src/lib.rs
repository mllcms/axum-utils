pub mod ip_addrs;
pub mod middleware;
pub mod res;
pub mod validation;

use res::Res;
use std::result;
pub type Result<T> = result::Result<Res<T>, Res<()>>;
