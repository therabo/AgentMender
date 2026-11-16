//! Open only the server-generated loopback URL with the system browser.
use std::net::{Ipv4Addr, SocketAddr};

pub(super) fn open(addr: SocketAddr, ticket: &str) -> std::io::Result<()> {
    if addr.ip() != Ipv4Addr::LOCALHOST
        || ticket.len() != 64
        || !ticket.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(std::io::Error::other("invalid browser launch address"));
    }
    let url = format!("http://{addr}/#ticket={ticket}");
    use std::process::{Command, Stdio};
    #[cfg(target_os = "macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    let mut child = command
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
