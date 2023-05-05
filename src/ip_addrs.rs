use colored::Colorize;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

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
            format!("Server runting at http://{}:{}/", ip, addr.port())
                .yellow()
                .bold()
        )
    }
}
