use colored::Colorize;
use std::{
    fs::{self, File},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
};

use crate::res::Res;
use std::result;

#[allow(dead_code)]
pub type Result<T> = result::Result<Res<T>, Res<()>>;

/// 输出服务地址 针对于 0.0.0.0 这种
#[allow(dead_code)]
pub fn echo_ip_addrs(addr: &SocketAddr) {
    let ip = addr.ip();
    let ips = if ip == IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)) {
        let mut iface = if_addrs::get_if_addrs().unwrap_or_default();
        iface.sort_by_key(|a| a.ip());
        iface
            .iter()
            .filter_map(|x| x.ip().is_ipv4().then_some(x.ip()))
            .collect()
    } else {
        vec![ip]
    };
    for ip in ips {
        println!(
            "{}",
            format!(
                "Server runting at {}",
                format!("http://{}:{}/", ip, addr.port()).underline()
            )
            .purple()
        )
    }
}

/// 创建日志文件并给定对应权限
pub fn create_log_file(path: String) -> File {
    if let Some(p) = Path::new(&path).parent() {
        fs::create_dir_all(p).expect("自动创建日志文件父级目录失败")
    }

    File::options()
        .create(true)
        .append(true)
        .write(true)
        .open(path)
        .expect("日志文件创建失败")
}
