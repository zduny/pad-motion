pub mod internals;

use internals::*;
use std::io::{Cursor, Result, Error, ErrorKind};
use byteorder::{WriteBytesExt, LittleEndian};

pub const PROTOCOL_VERSION: u16 = 1001;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MessageSource {
  Server,
  Client
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MessageType {
  ProtocolVersion,
  ConnectedControllers,
  ControllerData
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SlotState {
  NotConnected,
  Reserved,
  Connected
}

impl Default for SlotState {
  fn default() -> SlotState {
    SlotState::NotConnected
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DeviceType {
  NotApplicable,
  PartialGyro,
  FullGyro
}

impl Default for DeviceType {
  fn default() -> DeviceType {
    DeviceType::NotApplicable
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ConnectionType {
  NotApplicable,
  USB,
  Bluetooth
}

impl Default for ConnectionType {
  fn default() -> ConnectionType {
    ConnectionType::NotApplicable
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BatteryStatus {
  NotApplicable,
  Dying,
  Low,
  Medium,
  High,
  Full,
  Charging,
  Charged
}

impl Default for BatteryStatus {
  fn default() -> BatteryStatus {
    BatteryStatus::NotApplicable
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ControllerDataRequest {
  ReportAll,
  SlotNumber(u8),
  MAC(u64)
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MessagePayload {
  None,
  ProtocolVersion(u16),
  ConnectedControllersRequest { amount: i32, 
                                slot_numbers: [u8; 4] },
  ConnectedControllerResponse { controller_info: ControllerInfo },
  ControllerDataRequest(ControllerDataRequest),
  ControllerData { packet_number: u32,
                   controller_info: ControllerInfo,
                   controller_data: ControllerData }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MessageHeader {
  pub source: MessageSource,
  pub protocol_version: u16,
  pub message_length: u16,
  pub checksum: u32,
  pub source_id: u32
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ControllerInfo {
  pub slot: u8,
  pub slot_state: SlotState,
  pub device_type: DeviceType,
  pub connection_type: ConnectionType,
  pub mac_address: u64,
  pub battery_status: BatteryStatus
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct TouchData {
  active: bool,
  id: u8,
  position_x: u16,
  position_y: u16
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ControllerData {
  pub connected: bool,
  pub d_pad_left: bool,
  pub d_pad_down: bool,
  pub d_pad_right: bool,
  pub d_pad_up: bool,
  pub start: bool,
  pub right_stick_button: bool,
  pub left_stick_button: bool,
  pub select: bool,
  pub square: bool,
  pub cross: bool,
  pub circle: bool,
  pub triangle: bool,
  pub r1: bool,
  pub l1: bool,
  pub r2: bool,
  pub l2: bool,
  pub ps: u8,
  pub touch: u8,
  pub left_stick_x: u8,
  pub left_stick_y: u8,
  pub right_stick_x: u8,
  pub right_stick_y: u8,
  pub analog_d_pad_left: u8,
  pub analog_d_pad_down: u8,
  pub analog_d_pad_right: u8,
  pub analog_d_pad_up: u8,
  pub analog_square: u8,
  pub analog_triangle: u8,
  pub analog_cross: u8,
  pub analog_circle: u8,
  pub analog_r1: u8,
  pub analog_l1: u8,
  pub analog_r2: u8,
  pub analog_l2: u8,
  pub first_touch: TouchData,
  pub second_touch: TouchData,
  pub motion_data_timestamp: u64,
  pub accelerometer_x: f32,
  pub accelerometer_y: f32,
  pub accelerometer_z: f32,
  pub gyroscope_pitch: f32,
  pub gyroscope_yaw: f32,
  pub gyroscope_roll: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Message {
  pub header: MessageHeader,
  pub message_type: MessageType,
  pub payload: MessagePayload
}

fn compute_checksum(packet: &[u8]) -> u32 {
  let mut packet = packet.to_vec();
  for byte in &mut packet[8..12] {
      *byte = 0;
  }

  crc::crc32::checksum_ieee(&packet)
}

pub fn encode_message(writer: &mut Vec<u8>, message: Message) -> Result<()> {
  encode_message_header(writer, message.header)?;
  encode_message_type(writer, message.message_type)?;
  encode_message_payload(writer, message.payload)?;

  let length = (writer.len() - 16) as u16;
  let mut length_bytes = vec![];
  length_bytes.write_u16::<LittleEndian>(length)?;
  writer[6..8].swap_with_slice(&mut length_bytes[..]);

  let checksum = compute_checksum(writer);
  let mut checksum_bytes = vec![];
  checksum_bytes.write_u32::<LittleEndian>(checksum)?;
  writer[8..12].swap_with_slice(&mut checksum_bytes[..]);

  Ok(())
}

pub fn parse_message(message_source: MessageSource,
                     packet: &[u8], 
                     verify_checksum: bool) -> Result<Message> {
  let mut reader = Cursor::new(packet);
  let header = parse_message_header(&mut reader)?;

  if header.protocol_version != PROTOCOL_VERSION {
    return Err(Error::new(ErrorKind::InvalidData, "Unsupported protocol version"));
  }

  if packet.len() - 16 < header.message_length as usize {
    return Err(Error::new(ErrorKind::InvalidData, "Received packet is too short"));
  }

  if verify_checksum {
    let checksum = compute_checksum(packet);
    if checksum != header.checksum {
      return Err(Error::new(ErrorKind::InvalidData, "Packet has incorrect checksum"));
    }
  }

  let message_type = parse_message_type(&mut reader)?;

  let payload = parse_message_payload(&mut reader, message_source, message_type)?;

  Ok(Message {
    header,
    message_type,
    payload
  })
}
