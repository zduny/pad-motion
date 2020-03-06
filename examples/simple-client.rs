use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use pad_motion::protocol::*;
use pad_motion::client::*;

fn main() {
  let running = Arc::new(AtomicBool::new(true));

  {
    let running = running.clone();
    ctrlc::set_handler(move || {
      running.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
  }

  let client = Arc::new(Client::new(None, None, None).unwrap());
  let client_thread_join_handle = {
    let client = client.clone();
    client.start(running.clone())
  };

  client.request_connected_controllers_info(&[0, 1, 2, 3]).unwrap();
  while running.load(Ordering::SeqCst) {
    client.request_controller_data(ControllerDataRequest::ReportAll).unwrap();
    while let Some(event) = client.next_event() {
      println!("{:?}", event);
    }
    thread::sleep(Duration::from_secs(1));
  }

  client_thread_join_handle.join().unwrap();
}
