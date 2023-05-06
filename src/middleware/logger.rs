use std::{
    fs::File,
    io::Write,
    net::SocketAddr,
    sync::mpsc::{channel, Sender},
    task::{Context, Poll},
};

use axum::{body::Body, extract::ConnectInfo, http::Request, response::Response};
use chrono::{DateTime, Local};
use colored::Colorize;
use futures_util::future::BoxFuture;
use tower::{Layer, Service};

use crate::utils::create_log_file;

/// # Examples
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     let addr = "127.0.0.1:3000";
///     let app = Router::new().layer(Logger::default());
///
///     axum::Server::bind(&addr.parse().unwrap())
///         .serve(app.into_make_service_with_connect_info::<SocketAddr>())
///         .await
///         .unwrap();
/// }
/// ```
#[derive(Clone)]
pub struct Logger {
    sender: Sender<LogMsg>,
}

impl Logger {
    /// # Examples
    /// ```no_run
    /// Logger::new("logs/access/%Y-%m-%d.log", true, true);
    /// ```
    pub fn new(format: &str, stdout: bool, file_out: bool) -> Self {
        let mut time = Local::now();

        let mut file = file_out.then(|| {
            let path = time.format(format).to_string();
            create_log_file(path)
        });

        let (sender, rx) = channel::<LogMsg>();
        // 单独线程 同步写入日志
        let format = format.to_string();
        tokio::spawn(async move {
            for msg in rx {
                if stdout {
                    msg.stdout()
                }

                if let Some(file) = file.as_mut() {
                    // 切换日志文件
                    if time.date_naive() != msg.begin.date_naive() {
                        time = msg.begin;
                        *file = create_log_file(time.format(&format).to_string())
                    }
                    msg.file_out(file)
                }
            }
        });

        Self { sender }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("logs/access/%Y-%m-%d.log", true, true)
    }
}

impl<S> Layer<S> for Logger {
    type Service = LoggerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggerService {
            inner,
            sender: self.sender.clone(),
        }
    }
}

#[derive(Clone)]
pub struct LoggerService<S> {
    inner: S,
    sender: Sender<LogMsg>,
}

impl<S> Service<Request<Body>> for LoggerService<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let begin = Local::now();
        let method = req.method().to_string();
        let ip = match req.extensions().get::<ConnectInfo<SocketAddr>>() {
            Some(v) => v.0.ip().to_string(),
            None => panic!("Axum service 未配置 ConnectInfo<SocketAddr>"),
        };
        let path = req.uri().path().to_string();
        let sender = self.sender.clone();
        let future = self.inner.call(req);

        Box::pin(async move {
            let response: Self::Response = future.await?;
            let status = response.status().as_u16();

            let msg = LogMsg {
                logo: "[AXUM]".into(),
                begin,
                end: Local::now(),
                status,
                ip,
                method,
                path,
                other: "".into(),
            };

            if let Err(err) = sender.send(msg) {
                eprintln!("Send 日志时出现错误 -> {err}")
            }
            Ok(response)
        })
    }
}

struct LogMsg {
    logo: String,
    begin: DateTime<Local>,
    end: DateTime<Local>,
    status: u16,
    ip: String,
    method: String,
    path: String,
    other: String,
}

impl LogMsg {
    fn stdout(&self) {
        let status = match self.status / 100 {
            2 => format!(" {} ", self.status).on_green(),
            3 => format!(" {} ", self.status).on_blue(),
            4 | 5 => format!(" {} ", self.status).on_red(),
            _ => format!(" {} ", self.status).on_yellow(),
        };

        let method = match self.method.as_str() {
            "GET" | "POST" => format!(" {:<6} ", self.method).on_blue(),
            "DELETE" => format!(" {:<6} ", self.method).on_red(),
            _ => format!(" {:<6} ", self.method).on_yellow(),
        };

        println!(
            "[{}] {} |{}| {:>6} | {:>15} |{} {} {}",
            self.begin
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .truecolor(127, 132, 142),
            self.logo.bold().yellow(),
            status,
            format!("{}ms", (self.end - self.begin).num_milliseconds()),
            self.ip.yellow(),
            method,
            self.path,
            self.other
        );
    }

    fn file_out(&self, file: &mut File) {
        let msg = format!(
            "[{}] {} | {} | {:>6} | {:>15} | {:<6} {} {}\n",
            self.begin.format("%Y-%m-%d %H:%M:%S"),
            self.logo,
            self.status,
            format!("{}ms", (self.end - self.begin).num_milliseconds()),
            self.ip,
            self.method,
            self.path,
            self.other
        );
        if let Err(err) = file.write_all(msg.as_bytes()) {
            println!("日志写入文件时出错 -> {err}")
        }
    }
}
