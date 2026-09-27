//! Received jobs, in memory only (decision 3 in `.ai/stack.md`): the newest `max_jobs`
//! within `max_total_bytes`, oldest finished job dropped first. A job being received is
//! never dropped; when one would push the total over the limit, it is refused instead.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Receiving,
    /// The client closed the connection.
    Done,
    /// No byte for the idle timeout.
    IdleTimeout,
    /// Over the per-job or total size limit; the rest was not read.
    TooLarge,
    /// Reset or another read error.
    ConnectionError,
}

struct Job {
    id: u64,
    peer: SocketAddr,
    started_at: u64,
    ended_at: Option<u64>,
    state: JobState,
    bytes: Vec<u8>,
}

/// What the webview sees. Never the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct JobSummary {
    pub id: u64,
    pub peer: String,
    /// Unix ms.
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub state: JobState,
    pub size: usize,
}

impl Job {
    fn summary(&self) -> JobSummary {
        JobSummary {
            id: self.id,
            peer: self.peer.to_string(),
            started_at: self.started_at,
            ended_at: self.ended_at,
            state: self.state,
            size: self.bytes.len(),
        }
    }
}

/// `append` would take the job or the store over its size limit.
#[derive(Debug, PartialEq, Eq)]
pub struct TooLarge;

pub struct Jobs {
    jobs: VecDeque<Job>,
    total_bytes: usize,
    next_id: u64,
    max_jobs: usize,
    max_total_bytes: usize,
    max_job_bytes: usize,
}

impl Default for Jobs {
    fn default() -> Self {
        Self::new(100, 32 * 1024 * 1024, 16 * 1024 * 1024)
    }
}

impl Jobs {
    pub fn new(max_jobs: usize, max_total_bytes: usize, max_job_bytes: usize) -> Self {
        Self {
            jobs: VecDeque::new(),
            total_bytes: 0,
            next_id: 1,
            max_jobs,
            max_total_bytes,
            max_job_bytes,
        }
    }

    pub fn start(&mut self, peer: SocketAddr) -> JobSummary {
        let job = Job {
            id: self.next_id,
            peer,
            started_at: now_ms(),
            ended_at: None,
            state: JobState::Receiving,
            bytes: Vec::new(),
        };
        self.next_id += 1;
        let summary = job.summary();
        self.jobs.push_back(job);
        self.evict(0);
        summary
    }

    /// Adds received bytes to a job that is still receiving. Nothing is added on error.
    pub fn append(&mut self, id: u64, bytes: &[u8]) -> Result<(), TooLarge> {
        let Some(index) = self.index(id) else {
            return Ok(());
        };
        if self.jobs[index].state != JobState::Receiving {
            return Ok(());
        }
        if self.jobs[index].bytes.len() + bytes.len() > self.max_job_bytes {
            return Err(TooLarge);
        }
        if !self.evict(bytes.len()) {
            return Err(TooLarge);
        }
        // `evict` only removes finished jobs, so this one is still there, maybe shifted.
        let index = self.index(id).expect("a receiving job is never evicted");
        self.jobs[index].bytes.extend_from_slice(bytes);
        self.total_bytes += bytes.len();
        Ok(())
    }

    pub fn finish(&mut self, id: u64, state: JobState) -> Option<JobSummary> {
        let index = self.index(id)?;
        let job = &mut self.jobs[index];
        job.state = state;
        job.ended_at = Some(now_ms());
        Some(job.summary())
    }

    /// Oldest first.
    pub fn summaries(&self) -> Vec<JobSummary> {
        self.jobs.iter().map(Job::summary).collect()
    }

    fn index(&self, id: u64) -> Option<usize> {
        self.jobs.iter().position(|job| job.id == id)
    }

    /// Drops the oldest finished jobs until there is room for `incoming` more bytes and the
    /// count is within `max_jobs`. False if receiving jobs alone leave no room.
    fn evict(&mut self, incoming: usize) -> bool {
        while self.jobs.len() > self.max_jobs || self.total_bytes + incoming > self.max_total_bytes
        {
            let Some(index) = self
                .jobs
                .iter()
                .position(|job| job.state != JobState::Receiving)
            else {
                return self.total_bytes + incoming <= self.max_total_bytes;
            };
            let job = self.jobs.remove(index).expect("index from position");
            self.total_bytes -= job.bytes.len();
        }
        true
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> SocketAddr {
        "192.168.1.10:50000".parse().expect("valid address")
    }

    fn finished(jobs: &mut Jobs, bytes: &[u8]) -> u64 {
        let id = jobs.start(peer()).id;
        jobs.append(id, bytes).expect("fits");
        jobs.finish(id, JobState::Done);
        id
    }

    fn ids(jobs: &Jobs) -> Vec<u64> {
        jobs.summaries().iter().map(|job| job.id).collect()
    }

    #[test]
    fn keeps_bytes_and_summary() {
        let mut jobs = Jobs::default();
        let id = jobs.start(peer()).id;
        jobs.append(id, b"\x1b@").expect("fits");
        jobs.append(id, b"Hello").expect("fits");
        let summary = jobs.finish(id, JobState::Done).expect("job exists");
        assert_eq!(summary.size, 7);
        assert_eq!(summary.state, JobState::Done);
        assert_eq!(summary.peer, "192.168.1.10:50000");
        assert!(summary.ended_at.is_some());
    }

    #[test]
    fn drops_oldest_finished_job_over_count() {
        let mut jobs = Jobs::new(2, 1000, 1000);
        let first = finished(&mut jobs, b"a");
        let second = finished(&mut jobs, b"b");
        let third = finished(&mut jobs, b"c");
        assert_eq!(ids(&jobs), vec![second, third]);
        assert!(!ids(&jobs).contains(&first));
    }

    #[test]
    fn drops_oldest_finished_job_over_total_bytes() {
        let mut jobs = Jobs::new(10, 10, 10);
        let first = finished(&mut jobs, b"123456");
        let second = finished(&mut jobs, b"123456");
        assert_eq!(ids(&jobs), vec![second]);
        assert!(!ids(&jobs).contains(&first));
    }

    #[test]
    fn never_drops_a_receiving_job() {
        let mut jobs = Jobs::new(1, 1000, 1000);
        let receiving = jobs.start(peer()).id;
        let other = jobs.start(peer()).id;
        // Over the count, but both are receiving: both stay until one finishes.
        assert_eq!(ids(&jobs), vec![receiving, other]);
        jobs.finish(receiving, JobState::Done);
        jobs.append(other, b"x").expect("fits");
        assert_eq!(ids(&jobs), vec![other]);
    }

    #[test]
    fn refuses_a_job_over_its_own_limit() {
        let mut jobs = Jobs::new(10, 1000, 4);
        let id = jobs.start(peer()).id;
        jobs.append(id, b"1234").expect("exactly the limit fits");
        assert_eq!(jobs.append(id, b"5"), Err(TooLarge));
        assert_eq!(jobs.summaries()[0].size, 4);
    }

    #[test]
    fn refuses_when_receiving_jobs_fill_the_total() {
        let mut jobs = Jobs::new(10, 6, 6);
        let a = jobs.start(peer()).id;
        let b = jobs.start(peer()).id;
        jobs.append(a, b"1234").expect("fits");
        assert_eq!(jobs.append(b, b"123"), Err(TooLarge));
        jobs.append(b, b"12").expect("exactly the total fits");
    }

    #[test]
    fn append_to_an_unknown_or_finished_job_is_a_no_op() {
        let mut jobs = Jobs::default();
        assert_eq!(jobs.append(42, b"x"), Ok(()));
        assert!(jobs.summaries().is_empty());

        let id = finished(&mut jobs, b"a");
        assert_eq!(jobs.append(id, b"b"), Ok(()));
        assert_eq!(jobs.summaries()[0].size, 1);
    }
}
