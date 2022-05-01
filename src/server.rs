use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::io::Result;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::{JoinHandle, Thread};
use std::time::{Duration, Instant};

use crate::protocol::*;

#[derive(Copy, Clone, Debug, Default)]
struct Slot {
    controller_info: ControllerInfo,
    controller_data: ControllerData,
}

struct RequestedControllerData {
    packet_number: u32,
    slot_numbers: HashSet<u8>,
    mac_addresses: HashSet<u64>,
}

const DEFAULT_PORT: u16 = 26760;

pub trait DsServer {
    /// Starts background server thread.
    fn start(self, countinue_running: Arc<AtomicBool>, receiving_requests: Arc<AtomicBool>, parent: Thread) -> JoinHandle<()>;

    /// Update controller info (it will automatically send this data to connected clients).
    fn update_controller_info(&self, controller_info: ControllerInfo);

    /// Update controller data (it will automatically send this data to connected clients).
    fn update_controller_data(&self, slot_number: u8, controller_data: ControllerData);

    fn last_request_duration(&self) -> Duration;
}

pub struct Server {
    message_header: MessageHeader,
    slots: Mutex<[Slot; 4]>,
    connected_clients: Mutex<HashMap<SocketAddr, RequestedControllerData>>,
    socket: UdpSocket,
    last_request: Mutex<Instant>,
}

impl Server {
    /// Creates new server.
    ///
    /// # Arguments
    ///
    /// * `id` - server ID, pass `None` to use a random number.
    /// * `address` - server's UDP socket address, if `None` is passed `127.0.0.1:26760` is used.

    pub fn new(id: Option<u32>, address: Option<SocketAddr>) -> Result<Server> {
        let mut rng = rand::thread_rng();

        let server_id = match id {
            Some(id) => id,
            None => rng.gen(),
        };

        let message_header = {
            MessageHeader {
                source: MessageSource::Server,
                protocol_version: PROTOCOL_VERSION,
                message_length: 0,
                checksum: 0,
                source_id: server_id,
            }
        };

        let slots = {
            let mut slots: [Slot; 4] = [Default::default(); 4];
            for (i, slot) in slots.iter_mut().enumerate() {
                slot.controller_info.slot = i as u8;
            }

            Mutex::new(slots)
        };

        let connected_clients = Mutex::new(HashMap::new());

        let socket_address = match address {
            Some(address) => address,
            None => SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT)),
        };
        let socket = UdpSocket::bind(socket_address)?;
        socket.set_read_timeout(Some(Duration::from_secs_f64(0.2)))?;
        socket.set_write_timeout(Some(Duration::from_secs_f64(0.2)))?;

        Ok(Server {
            message_header,
            slots,
            connected_clients,
            socket,
            last_request: Mutex::new(Instant::now()),
        })
    }

    fn encode_and_send(&self, target: SocketAddr, message: Message) -> Result<()> {
        let mut encoded_message = vec![];
        encode_message(&mut encoded_message, message).unwrap();

        self.socket
            .send_to(&encoded_message, target)
            .map(|_amount| ())
    }

    fn send_protocol_version(&self, target: SocketAddr) -> Result<()> {
        let message = Message {
            header: self.message_header,
            message_type: MessageType::ConnectedControllers,
            payload: MessagePayload::ProtocolVersion(PROTOCOL_VERSION),
        };

        self.encode_and_send(target, message)
    }

    fn send_connected_controller_info(&self, target: SocketAddr, slot_number: u8) -> Result<()> {
        let controller_info = self.slots.lock().unwrap()[slot_number as usize].controller_info;

        let payload = MessagePayload::ConnectedControllerResponse { controller_info };

        let message = Message {
            header: self.message_header,
            message_type: MessageType::ConnectedControllers,
            payload,
        };

        self.encode_and_send(target, message)
    }

    fn send_slot_data(
        &self,
        target: SocketAddr,
        slot: Slot,
        packet_number: &mut u32,
    ) -> Result<()> {
        let payload = MessagePayload::ControllerData {
            packet_number: *packet_number,
            controller_info: slot.controller_info,
            controller_data: slot.controller_data,
        };

        let message = Message {
            header: self.message_header,
            message_type: MessageType::ControllerData,
            payload,
        };

        let result = self.encode_and_send(target, message);
        if result.is_ok() {
            *packet_number += 1;
        }

        result
    }

    fn send_controller_data(&self) -> Result<()> {
        let slots = self.slots.lock().unwrap();
        let mut connected_clients = self.connected_clients.lock().unwrap();

        connected_clients.retain(|&client_address, requested_controller_data| {
            let mut already_sent = HashSet::new();

            for &slot_number in requested_controller_data.slot_numbers.iter() {
                let slot = slots[slot_number as usize];
                let result = self.send_slot_data(
                    client_address,
                    slot,
                    &mut requested_controller_data.packet_number,
                );

                if result.is_ok() {
                    already_sent.insert(slot_number);
                } else {
                    return false;
                }
            }

            for &mac_address in requested_controller_data.mac_addresses.iter() {
                let slot_number = slots
                    .iter()
                    .position(|slot| slot.controller_info.mac_address == mac_address);
                if let Some(slot_number) = slot_number {
                    if !already_sent.contains(&(slot_number as u8)) {
                        let slot = slots[slot_number];
                        let result = self.send_slot_data(
                            client_address,
                            slot,
                            &mut requested_controller_data.packet_number,
                        );

                        if result.is_ok() {
                            already_sent.insert(slot_number as u8);
                        } else {
                            return false;
                        }
                    }
                }
            }

            true
        });

        Ok(())
    }

    fn handle_request(&self, source: SocketAddr, request: Message) -> Result<()> {
        match request.message_type {
            MessageType::ProtocolVersion => self.send_protocol_version(source),
            _ => {
                match request.payload {
                    MessagePayload::ConnectedControllersRequest {
                        amount,
                        slot_numbers,
                    } => {
                        for i in 0..amount {
                            let slot_number = slot_numbers[i as usize];
                            self.send_connected_controller_info(source, slot_number)?;
                        }

                        Ok(())
                    }
                    MessagePayload::ControllerDataRequest(request) => {
                        {
                            *self.last_request.lock().unwrap() = Instant::now();
                            let mut connected_clients = self.connected_clients.lock().unwrap();
                            let requested = connected_clients.entry(source).or_insert(
                                RequestedControllerData {
                                    packet_number: 0,
                                    slot_numbers: HashSet::new(),
                                    mac_addresses: HashSet::new(),
                                },
                            );

                            match request {
                                ControllerDataRequest::ReportAll => {
                                    requested.slot_numbers.insert(0);
                                    requested.slot_numbers.insert(1);
                                    requested.slot_numbers.insert(2);
                                    requested.slot_numbers.insert(3);
                                }
                                ControllerDataRequest::SlotNumber(slot_number) => {
                                    requested.slot_numbers.insert(slot_number);
                                }
                                ControllerDataRequest::MAC(mac) => {
                                    requested.mac_addresses.insert(mac);
                                }
                            };
                        }

                        self.send_controller_data()
                    }
                    _ => Ok(()), // ignore request
                }
            }
        }
    }
}

impl DsServer for Arc<Server> {
    fn start(self, countinue_running: Arc<AtomicBool>, receiving_requests: Arc<AtomicBool>, parent: Thread) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let mut buf = [0_u8; 100];
            while countinue_running.load(Ordering::SeqCst) {
                if let Ok((amount, source)) = self.socket.recv_from(&mut buf) {
                    let message = parse_message(MessageSource::Client, &buf[..amount], true);
                    if let Ok(message) = message {
                        if !receiving_requests.load(Ordering::SeqCst) {
                            receiving_requests.store(true, Ordering::SeqCst);
                            parent.unpark();
                        };
                        let _ = self.handle_request(source, message);
                    }
                }
            }
        })
    }

    fn update_controller_info(&self, controller_info: ControllerInfo) {
        assert!(controller_info.slot < 4);

        let slot_number = controller_info.slot;
        {
            let mut slots = self.slots.lock().unwrap();
            slots[slot_number as usize].controller_info = controller_info;
        }

        let connected_clients = self.connected_clients.lock().unwrap();
        for &address in connected_clients.keys() {
            let _ = self.send_connected_controller_info(address, slot_number);
        }
    }

    fn update_controller_data(&self, slot_number: u8, controller_data: ControllerData) {
        assert!(slot_number < 4);

        {
            let mut slots = self.slots.lock().unwrap();
            slots[slot_number as usize].controller_data = controller_data;
        }

        let _ = self.send_controller_data();
    }

    fn last_request_duration(&self) -> Duration {
        {*self.last_request.lock().unwrap()}.elapsed()
    }
}
