use std::process::Stdio;
use tokio::process::{ChildStdout, Command};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

#[derive(Debug)]
pub struct ChildProcessError<X: 'static> {
    pub message: String,
    pub x: X,
}

impl <X: 'static> ChildProcessError<X> {
    pub fn new(message: String, x: X) -> Self {
        Self {
            message,
            x,
        }
    }
}

pub fn start_child_process<X: 'static, O: 'static>(
    cmd: &str,
    args: &[String],
    x: X,
    stream_handler: impl Fn(ChildStdout, X) -> O,
    cancel_token: CancellationToken,
) -> Result<O, ChildProcessError<X>> {
    debug!("spawning child process ... [{} {}]", cmd, args.join(" "));
    let mut child = match Command::new(cmd)
        .args(args)
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return Err(ChildProcessError::new(
                format!("error starting child process: {:?}", err),
                x,
            ));
        }
    };

    let child_out = match child.stdout.take() {
        Some(child_out) => child_out,
        None => {
            return Err(ChildProcessError::new(
                "Unable to open stdout stream from child priocess".to_owned(),
                x,
            ));
        }
    };

    debug!("creating stream handler for child process ...");
    let output = stream_handler(child_out, x);

    debug!("waiting for child process to exit ...");
    tokio::spawn(async move {
        tokio::select! {
            child_res = child.wait() => {
                error!("child process exited unexpectedly! {:?}", child_res);
            },
            _ = cancel_token.cancelled() => {
                debug!("killing child process ...");
                child.kill().await.unwrap();
            }
        }
    });

    Ok(output)
}
