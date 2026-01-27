use crate::app::SamplerOps;
use crate::config::{AppConfig, LockingStrategy};
use crate::errors::AppError;
use crate::priority::SpiedRecordQueueMap;
use crate::sample::StackTrace;
use anyhow::Error;
use futures::TryStreamExt;
use remoteprocess::Pid;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, RwLock};
use tokio_util::codec::{Encoder, FramedRead, LengthDelimitedCodec};
use tracing::{debug, info, instrument, trace};

/// Configuration passed to the subprocess worker via stdin
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub pid: Pid,
    pub sampling_rate: u64,
    pub subprocesses: bool,
    pub native: bool,
    pub dump_locals: u64,
    pub locking_strategy: LockingStrategy,
}

impl WorkerConfig {
    pub fn new(config: &AppConfig, pid: Pid) -> Self {
        WorkerConfig {
            pid,
            sampling_rate: config.sampling_rate,
            subprocesses: config.subprocesses,
            native: config.native,
            dump_locals: config.dump_locals,
            locking_strategy: config.locking_strategy,
        }
    }
}

/// A sampler that runs py-spy in a subprocess and communicates via serialized traces.
pub struct SubprocessSampler {
    child: Child,
}

impl SubprocessSampler {
    /// Spawn a subprocess sampler for the given PID.
    #[instrument(skip(config), fields(pid = pid))]
    pub fn new(config: &AppConfig, pid: Pid) -> Result<Self, Error> {
        let exe = std::env::current_exe()?;
        debug!(?exe, "Spawning subprocess worker");

        let mut child = Command::new(exe)
            .arg("subprocess-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Send config via stdin as length-delimited bincode
        let worker_config = WorkerConfig::new(config, pid);
        let data = bincode::serialize(&worker_config)?;
        debug!(config_size = data.len(), "Sending config to subprocess");

        let mut stdin = child.stdin.take().ok_or(AppError::SamplerSenderError)?;
        let mut codec = LengthDelimitedCodec::new();
        let mut buf = tokio_util::bytes::BytesMut::new();
        codec.encode(data.into(), &mut buf)?;
        stdin.write_all(&buf)?;
        stdin.flush()?;
        drop(stdin); // Close stdin to signal we're done

        info!("Subprocess spawned successfully");
        Ok(SubprocessSampler { child })
    }
}

impl Drop for SubprocessSampler {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl SamplerOps for SubprocessSampler {
    fn from_config_and_id(config: &AppConfig, pid: Pid) -> Result<Self, Error> {
        SubprocessSampler::new(config, pid)
    }

    #[instrument(skip_all)]
    async fn push_to_queue(
        mut self,
        record_queue_map: Arc<RwLock<SpiedRecordQueueMap>>,
    ) -> Result<(), Error> {
        let stdout = self
            .child
            .stdout
            .take()
            .ok_or(AppError::SamplerSenderError)?;

        // Convert std stdout to tokio async reader
        let stdout = tokio::io::BufReader::new(tokio::process::ChildStdout::from_std(stdout)?);

        let mut length_delimited = FramedRead::new(stdout, LengthDelimitedCodec::new());

        debug!("Waiting for traces from subprocess");
        let mut trace_count = 0u64;
        while let Some(frame) = length_delimited.try_next().await? {
            trace!(frame_size = frame.len(), "Received frame");
            let trace: StackTrace = bincode::deserialize(&frame)?;
            record_queue_map
                .write()
                .map_err(|_| AppError::SamplerSenderError)?
                .increment(&trace);
            trace_count += 1;
        }

        info!(trace_count, "Subprocess stream ended");
        Ok(())
    }
}

impl From<LockingStrategy> for py_spy::config::LockingStrategy {
    fn from(strategy: LockingStrategy) -> Self {
        match strategy {
            LockingStrategy::Lock => py_spy::config::LockingStrategy::Lock,
            LockingStrategy::NonBlocking => py_spy::config::LockingStrategy::NonBlocking,
            LockingStrategy::AlreadyLocked => py_spy::config::LockingStrategy::AlreadyLocked,
        }
    }
}

/// Entry point for the subprocess worker
#[instrument]
pub fn run_worker() -> Result<(), Error> {
    use std::io::Read;
    use tokio_util::bytes::BytesMut;
    use tokio_util::codec::Decoder;

    debug!("Reading config from stdin");

    // Read config from stdin (length-delimited bincode)
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let mut input_buf = Vec::new();
    stdin.read_to_end(&mut input_buf)?;

    debug!(bytes = input_buf.len(), "Received config data");

    let mut codec = LengthDelimitedCodec::new();
    let mut bytes = BytesMut::from(input_buf.as_slice());
    let frame = codec
        .decode(&mut bytes)?
        .ok_or_else(|| anyhow::anyhow!("No config received on stdin"))?;
    let config: WorkerConfig = bincode::deserialize(&frame)?;

    info!(
        pid = config.pid,
        sampling_rate = config.sampling_rate,
        "Config decoded, creating sampler"
    );

    let sampler = py_spy::sampler::Sampler::new(
        config.pid,
        &py_spy::Config {
            blocking: config.locking_strategy.into(),
            sampling_rate: config.sampling_rate,
            subprocesses: config.subprocesses,
            native: config.native,
            dump_locals: config.dump_locals,
            ..Default::default()
        },
    )?;

    info!("Sampler created, starting trace collection");

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let mut codec = LengthDelimitedCodec::new();
    let mut buf = BytesMut::new();

    let mut sample_count = 0u64;
    let mut trace_count = 0u64;
    for sample in sampler {
        sample_count += 1;
        trace!(sample_count, traces = sample.traces.len(), "Got sample");
        for trace in sample.traces.iter() {
            let owned_trace: StackTrace = trace.into();
            let data = bincode::serialize(&owned_trace)?;

            codec.encode(data.into(), &mut buf)?;
            stdout.write_all(&buf)?;
            stdout.flush()?;
            buf.clear();
            trace_count += 1;
        }
    }

    info!(sample_count, trace_count, "Sampling completed");
    Ok(())
}
