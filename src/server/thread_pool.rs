
use std::thread;
use std::sync::mpsc::{ channel, Sender, Receiver };
use std::sync::{ Arc, Mutex };

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: Option<Sender<Job>>,
}

impl ThreadPool {
  pub fn new(num: usize) -> Self {
    assert!(num > 0);
    let mut workers = Vec::with_capacity(num);
    let (sender, receiver) = channel();
    let receiver = Arc::new(Mutex::new(receiver));
    for id in 0..num {
      let worker = Worker::new(id, Arc::clone(&receiver));
      workers.push(worker);
    }
    ThreadPool { workers, sender: Some(sender) }
  }

  pub fn execute<T>(&self, f: T)
  where
    T: FnOnce() + Send + 'static {
    let job = Box::new(f);
    self.sender.as_ref().unwrap().send(job).unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    drop(self.sender.take());
    for worker in &mut self.workers {
      if let Some(thread) = worker.thread.take() {
        println!("thread {} shutting down", worker.id);
        thread.join().unwrap();
      }
    }
  }
}

struct Worker {
  thread: Option<thread::JoinHandle<()>>,
  id: usize,
}

impl Worker {
  fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Self {
    let thread = thread::spawn(move || {
      loop {
        let result = receiver.lock().unwrap().recv();
        match result {
          Ok(job) => {
            println!("thread {} running", id);
            job();
          }
          Err(_) => {
            println!("disconnected");
            break;
          }
        }
      }
    });

    Worker { thread: Some(thread), id }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_thread_pool() {
    let pool = ThreadPool::new(4);
    pool.execute(|| {
      println!("---\n execute \n---");
    });
    assert_eq!(pool.workers.len(), 4);
  }
}