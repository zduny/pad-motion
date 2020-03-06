use std::thread::JoinHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Result;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;
use rand::Rng;
use crossbeam_queue::ArrayQueue;
use crate::protocol::*;

#[derive(Copy, Clone, Debug, Default)]
struct Slot {
  controller_info: ControllerInfo,
  controller_data: ControllerData,
  latest_packet_number: u32
}

#[derive(Copy, Clone, Debug)]
pub enum ClientEvent {
  ControllerInfoChanged(ControllerInfo),
  ControllerDataChanged {
    controller_info: ControllerInfo,
    controller_data: ControllerData
  }
}

const DEFAULT_PORT: u16 = 3333;
const DEFAULT_SERVER_PORT: u16 = 26760;

pub trait DsClient {
  /// Starts background client thread.
  fn start(self, countinue_running: Arc<AtomicBool>) -> JoinHandle<()>;

  /// Gets currently cached controller info for given slot number.
  fn get_controller_info(&self, slot_number: u8) -> ControllerInfo;

  /// Gets currently cached controller data for given slot number.
  fn get_controller_data(&self, slot_number: u8) -> ControllerData;

  /// Returns next event in event queue or `None` if empty.
  fn next_event(&self) -> Option<ClientEvent>;
}

pub struct Client {
  server_address: SocketAddr,
  message_header: MessageHeader,
  slots: Mutex<[Slot; 4]>,
  socket: UdpSocket,
  events: ArrayQueue<ClientEvent>
}

impl Client {
  /// Creates new client.
  /// 
  /// # Arguments
  /// 
  /// * `id` - client ID, pass `None` to use a random number.
  /// * `address` - client's UDP socket address, if `None` is passed `127.0.0.1:3333` is used.
  /// * `server_address` - server's UDP socket address, the default (if `None` is passed) is `127.0.0.1:267601`.
  pub fn new(id: Option<u32>, address: Option<SocketAddr>, server_address: Option<SocketAddr>) -> Result<Client> {
    let mut rng = rand::thread_rng();

    let client_id = match id {
      Some(id) => id,
      None => rng.gen()
    };

    let message_header = {
      MessageHeader {
        source: MessageSource::Client,
        protocol_version: PROTOCOL_VERSION,
        message_length: 0,
        checksum: 0,
        source_id: client_id
      }
    };

    let slots = {
      let mut slots: [Slot; 4] = [Default::default(); 4];
      let mut i = 0;
      for slot in slots.iter_mut() {
        slot.controller_info.slot = i;
        i += 1;
      }

      Mutex::new(slots)
    };

    let client_address = match address {
      Some(address) => address,
      None => SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT))
    };

    let server_address = match server_address {
      Some(address) => address,
      None => SocketAddr::from(([127, 0, 0, 1], DEFAULT_SERVER_PORT))
    };
    let socket = UdpSocket::bind(client_address)?;
    socket.set_read_timeout(Some(Duration::from_secs_f64(0.2)))?;
    socket.set_write_timeout(Some(Duration::from_secs_f64(0.2)))?;

    let events = ArrayQueue::new(50);

    Ok(Client {
      server_address,
      message_header,
      slots,
      socket,
      events
    })
  }

  fn encode_and_send(&self, message: Message) -> Result<()> {
    let mut encoded_message = vec![];
    encode_message(&mut encoded_message, message).unwrap();
  
    self.socket.send_to(&encoded_message, self.server_address).map(|_amount| ())
  }

  /// Ask server to send controller info for given slot numbers.
  /// 
  /// # Arguments
  /// 
  /// * `slot_numbers` - slot numbers to ask info for, must contain at most 4 elements.
  pub fn request_connected_controllers_info(&self, slot_numbers: &[u8]) -> Result<()> {
    let slot_numbers = {
      let mut slots = [0; 4];

      let mut i = 0;
      for &slot in slot_numbers {
        slots[i] = slot;
        i += 1;
      }

      slots
    };

    let payload = MessagePayload::ConnectedControllersRequest {
      amount: slot_numbers.len() as i32,
      slot_numbers
    };

    let message = Message {
      header: self.message_header,
      message_type: MessageType::ConnectedControllers,
      payload
    };

    self.encode_and_send(message)
  }

  /// Ask server to send controller data for given slot numbers.
  /// You must call this method periodically if you want server to send data.
  pub fn request_controller_data(&self, request: ControllerDataRequest) -> Result<()> {
    let payload = MessagePayload::ControllerDataRequest(request);

    let message = Message {
      header: self.message_header,
      message_type: MessageType::ControllerData,
      payload
    };

    self.encode_and_send(message)
  }

  fn handle_response(&self, response: Message) -> Option<ClientEvent> {
    match response.message_type {
      MessageType::ProtocolVersion => None,
      _ => {
        match response.payload {
          MessagePayload::ConnectedControllerResponse { controller_info } => {
            let slot_number = controller_info.slot;

            let mut slots = self.slots.lock().unwrap();
            if slots[slot_number as usize].controller_info != controller_info {
              slots[slot_number as usize].controller_info = controller_info;

              let event = ClientEvent::ControllerInfoChanged(controller_info);

              Some(event)
            } else {
              None
            }
          },
          MessagePayload::ControllerData { packet_number,
                                           controller_info,
                                           controller_data } => {
            let slot_number = controller_info.slot;

            let mut slots = self.slots.lock().unwrap();
            
            let slot = slots[slot_number as usize];
            if packet_number > slot.latest_packet_number {
              slots[slot_number as usize].latest_packet_number = packet_number;

              if slot.controller_info != controller_info || slot.controller_data != controller_data {
                slots[slot_number as usize].controller_info = controller_info;
                slots[slot_number as usize].controller_data = controller_data;
  
                let event = ClientEvent::ControllerDataChanged {
                  controller_info,
                  controller_data
                };
  
                Some(event)
              } else {
                None
              }
            } else {
              None
            }
          }
          _ => None // ignore response
        }
      }
    }
  }
}

impl DsClient for Arc<Client> {
  fn start(self, countinue_running: Arc<AtomicBool>) -> JoinHandle<()> {
    let countinue_running = countinue_running.clone();

    std::thread::spawn(move || {
      let mut buf = [0 as u8; 100];
      while countinue_running.load(Ordering::SeqCst) {
        match self.socket.recv_from(&mut buf) {
          Ok((amount, source)) => {
            if source == self.server_address {
              let message = parse_message(MessageSource::Server, &buf[..amount], true);
              if let Ok(message) = message {
                let event = self.handle_response(message);
                if let Some(event) = event {
                  let _ = self.events.push(event);
                }
              }
            }
          },
          _ => ()
        }
      }
    })
  }

  fn get_controller_info(&self, slot_number: u8) -> ControllerInfo {
    assert!(slot_number < 4);

    let slot = self.slots.lock().unwrap()[slot_number as usize];

    slot.controller_info
  }

  fn get_controller_data(&self, slot_number: u8) -> ControllerData {
    assert!(slot_number < 4);

    let slot = self.slots.lock().unwrap()[slot_number as usize];

    slot.controller_data
  }

  fn next_event(&self) -> Option<ClientEvent> {
    self.events.pop().ok()
  }
}
