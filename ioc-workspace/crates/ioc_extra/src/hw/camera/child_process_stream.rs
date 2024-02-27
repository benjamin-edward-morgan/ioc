
use std::process::Stdio;
use tokio::process::{Command, ChildStdout};
use futures::Future;
use tracing::{error, info};

#[derive(Debug)]
pub struct ChildProcessError {
   message: String,
}

impl ChildProcessError {
    pub fn new(message: &str) -> Self {
        Self{ message: message.to_string() }
    } 
}

impl From<std::io::Error> for ChildProcessError {
    fn from(value: std::io::Error) -> Self {
        Self{ message: format!("{:?}", value) }
    }
}

pub fn start_child_process<O>(
    cmd: &str,
    args: &[&str],
    stream_handler: fn(ChildStdout) -> O,
    kill_switch: impl Future<Output = ()> + Send + 'static,
) -> Result<O, ChildProcessError>
{
    info!("Spawing child process... [{} {}]", cmd, args.join(" "));

    let mut child = Command::new(cmd)
        .args(args)
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()?;

    let child_out = child.stdout.take()
        .ok_or(ChildProcessError::new(
            "Unable to open stdout stream from child priocess"
        ))?;

    info!("creating stream handler for child process...");
    let output = stream_handler(child_out);

    info!("spawning task to wait for child...");
    tokio::spawn(async move {
        tokio::select! {
            child_res = child.wait() => {
                error!("child exited unexpectedly! {:?}", child_res);
            },
            _ = kill_switch => {
                info!("killing child {:?}", child);
                child.kill().await.unwrap();
                info!("done killing child");
            }
        }        
    });
    
    info!("returning child process ouput");
    Ok(output)
}