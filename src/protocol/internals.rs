use super::*;
use std::io::{Cursor, Result, Error, ErrorKind};
use std::io::prelude::*;
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};

fn invalid_data_error(message: &str) -> Error {
  Error::new(ErrorKind::InvalidData, message)
}

pub fn encode_message_header(writer: &mut Vec<u8>, message_header: MessageHeader) -> Result<()> {
  match message_header.source {
      MessageSource::Server => writer.write(b"DSUS")?,
      MessageSource::Client => writer.write(b"DSUC")?
  };

  writer.write_u16::<LittleEndian>(message_header.protocol_version)?;
  writer.write_u16::<LittleEndian>(message_header.message_length)?;
  writer.write_u32::<LittleEndian>(message_header.checksum)?;
  writer.write_u32::<LittleEndian>(message_header.source_id)
}

pub fn parse_message_header(reader: &mut Cursor<&[u8]>) -> Result<MessageHeader> {
  let source = {
      let mut buffer = [0 as u8; 4];
      reader.read(&mut buffer)?;
      let magic_string = {
          let string = std::str::from_utf8(&buffer);

          match string {
              Ok(str) => Ok(str),
              _ => Err(invalid_data_error("Magic string is not a valid UTF-8 string"))
          }
      }?;

      match magic_string {
          "DSUS" => Ok(MessageSource::Server), 
          "DSUC" => Ok(MessageSource::Client),
          _ => Err(invalid_data_error("Unrecognized magic string"))
      }?
  };

  let protocol_version = reader.read_u16::<LittleEndian>()?;
  let message_length = reader.read_u16::<LittleEndian>()?;
  let checksum = reader.read_u32::<LittleEndian>()?;
  let source_id = reader.read_u32::<LittleEndian>()?;

  Ok(MessageHeader {
      source,
      protocol_version,
      message_length,
      checksum,
      source_id
  })
}

pub fn encode_message_type(writer: &mut Vec<u8>, message_type: MessageType) -> Result<()> {
  let encoded = match message_type {
      MessageType::ProtocolVersion => 0x100000,
      MessageType::ConnectedControllers => 0x100001,
      MessageType::ControllerData => 0x100002,
  };

  writer.write_u32::<LittleEndian>(encoded)
}

pub fn parse_message_type(reader: &mut Cursor<&[u8]>) -> Result<MessageType> {
  let message_type = {
      let message_type = reader.read_u32::<LittleEndian>()?;

      match message_type {
          0x100000 => Ok(MessageType::ProtocolVersion),
          0x100001 => Ok(MessageType::ConnectedControllers),
          0x100002 => Ok(MessageType::ControllerData),
          _ => Err(invalid_data_error("Invalid message type"))
      }
  }?;

  Ok(message_type)
}

pub fn encode_controller_info(writer: &mut Vec<u8>, controller_info: ControllerInfo) -> Result<()> {
  writer.write_u8(controller_info.slot)?;

  let slot_state = match controller_info.slot_state {
      SlotState::NotConnected => 0,
      SlotState::Reserved =>     1,
      SlotState::Connected =>    2
  };
  writer.write_u8(slot_state)?;

  let device_type = match controller_info.device_type {
      DeviceType::NotApplicable => 0,
      DeviceType::PartialGyro =>   1,
      DeviceType::FullGyro =>      2
  };
  writer.write_u8(device_type)?;

  let connection_type = match controller_info.connection_type {
      ConnectionType::NotApplicable => 0,
      ConnectionType::USB =>           1,
      ConnectionType::Bluetooth =>     2
  };
  writer.write_u8(connection_type)?;

  writer.write_u48::<LittleEndian>(controller_info.mac_address)?;

  let battery_status = match controller_info.battery_status {
      BatteryStatus::NotApplicable => 0x00,
      BatteryStatus::Dying =>         0x01,
      BatteryStatus::Low =>           0x02,
      BatteryStatus::Medium =>        0x03,
      BatteryStatus::High =>          0x04,
      BatteryStatus::Full =>          0x05,
      BatteryStatus::Charging =>      0xEE,
      BatteryStatus::Charged =>       0xEF
  };
  writer.write_u8(battery_status)
}

pub fn parse_controller_info(reader: &mut Cursor<&[u8]>) -> Result<ControllerInfo> {
  let slot = reader.read_u8()?;
  if slot > 4 {
      return Err(invalid_data_error("Invalid slot number"));
  }

  let slot_state = {
      let slot_state = reader.read_u8()?;

      match slot_state {
          0 => Ok(SlotState::NotConnected),
          1 => Ok(SlotState::Reserved),
          2 => Ok(SlotState::Connected),
          _ => Err(invalid_data_error("Invalid slot number"))
      }
  }?;

  let device_type = {
      let device_type = reader.read_u8()?;

      match device_type {
          0 => Ok(DeviceType::NotApplicable),
          1 => Ok(DeviceType::PartialGyro),
          2 => Ok(DeviceType::FullGyro),
          _ => Err(invalid_data_error("Invalid device type"))
      }
  }?;

  let connection_type = {
      let connection_type = reader.read_u8()?;

      match connection_type {
          0 => Ok(ConnectionType::NotApplicable),
          1 => Ok(ConnectionType::USB),
          2 => Ok(ConnectionType::Bluetooth),
          _ => Err(invalid_data_error("Invalid connection type"))
      }
  }?;

  let mac_address = reader.read_u48::<LittleEndian>()?;

  let battery_status =  {
      let battery_status = reader.read_u8()?;

      match battery_status {
          0x00 => Ok(BatteryStatus::NotApplicable),
          0x01 => Ok(BatteryStatus::Dying),
          0x02 => Ok(BatteryStatus::Low),
          0x03 => Ok(BatteryStatus::Medium),
          0x04 => Ok(BatteryStatus::High),
          0x05 => Ok(BatteryStatus::Full),
          0xEE => Ok(BatteryStatus::Charging),
          0xEF => Ok(BatteryStatus::Charged),
          _ => Err(invalid_data_error("Invalid battery status"))
      }
  }?;

  Ok(ControllerInfo {
      slot,
      slot_state,
      device_type,
      connection_type,
      mac_address,
      battery_status
  })
}

pub fn encode_touch_data(writer: &mut Vec<u8>, touch_data: TouchData) -> Result<()> {
  match touch_data.active {
    false => writer.write_u8(0),
    true =>  writer.write_u8(1),
  }?;

  writer.write_u8(touch_data.id)?;

  writer.write_u16::<LittleEndian>(touch_data.position_x)?;
  writer.write_u16::<LittleEndian>(touch_data.position_y)
}

pub fn parse_touch_data(reader: &mut Cursor<&[u8]>) -> Result<TouchData> {
  let active = {
    let active = reader.read_u8()?;

    match active {
      0 => Ok(false),
      1 => Ok(true),
      _ => Err(invalid_data_error("Invalid touch active value"))
    }
  }?;

  let id = reader.read_u8()?;

  let position_x = reader.read_u16::<LittleEndian>()?;
  let position_y = reader.read_u16::<LittleEndian>()?;

  Ok(TouchData {
    active,
    id,
    position_x,
    position_y
  })
}

pub fn encode_controller_data_request(writer: &mut Vec<u8>, 
                                      request: ControllerDataRequest) -> Result<()> {
  match request {
    ControllerDataRequest::ReportAll => {
      writer.write_u8(0)?;
      writer.write_u8(0)?;
      writer.write_u48::<LittleEndian>(0)
    },
    ControllerDataRequest::SlotNumber(slot) => {
      writer.write_u8(1)?;
      writer.write_u8(slot)?;
      writer.write_u48::<LittleEndian>(0)
    },
    ControllerDataRequest::MAC(mac) => {
      writer.write_u8(2)?;
      writer.write_u8(0)?;
      writer.write_u48::<LittleEndian>(mac)
    }
  }
}

pub fn parse_controller_data_request(reader: &mut Cursor<&[u8]>) -> Result<ControllerDataRequest> {
  let request_type = reader.read_u8()?;

  match request_type {
    0 => Ok(ControllerDataRequest::ReportAll),
    1 => {
      let slot_number = reader.read_u8()?;
      if slot_number >= 4 {
        return Err(invalid_data_error("Invalid slot number requested"))
      }

      Ok(ControllerDataRequest::SlotNumber(slot_number))
    }
    2 => {
      let mac_address = reader.read_u48::<LittleEndian>()?;

      Ok(ControllerDataRequest::MAC(mac_address))
    },
    _ => Err(invalid_data_error("Invalid controller data request type"))
  }
}

fn bit_array_to_u8(input: [bool; 8]) -> u8 {
  let mut result = 0;

  result |= (input[0] as u8) * 0b10000000;
  result |= (input[1] as u8) * 0b01000000;
  result |= (input[2] as u8) * 0b00100000;
  result |= (input[3] as u8) * 0b00010000;
  result |= (input[4] as u8) * 0b00001000;
  result |= (input[5] as u8) * 0b00000100;
  result |= (input[6] as u8) * 0b00000010;
  result |= (input[7] as u8) * 0b00000001;

  result
}

pub fn encode_controller_data(writer: &mut Vec<u8>, 
                              packet_number: u32,
                              controller_data: ControllerData) -> Result<()> {
  let connected = match controller_data.connected {
    false => 0,
    true => 1
  };
  writer.write_u8(connected)?;

  writer.write_u32::<LittleEndian>(packet_number)?;

  let button_data = [controller_data.d_pad_left,
                     controller_data.d_pad_down,
                     controller_data.d_pad_right,
                     controller_data.d_pad_up,
                     controller_data.start,
                     controller_data.right_stick_button,
                     controller_data.left_stick_button,
                     controller_data.select];
  writer.write_u8(bit_array_to_u8(button_data))?;

  let button_data = [controller_data.square,
                     controller_data.cross,
                     controller_data.circle,
                     controller_data.triangle,
                     controller_data.r1,
                     controller_data.l1,
                     controller_data.r2,
                     controller_data.l2];
  writer.write_u8(bit_array_to_u8(button_data))?;

  writer.write_u8(controller_data.ps)?;

  writer.write_u8(controller_data.touch)?;

  writer.write_u8(controller_data.left_stick_x)?;
  writer.write_u8(controller_data.left_stick_y)?;

  writer.write_u8(controller_data.right_stick_x)?;
  writer.write_u8(controller_data.right_stick_y)?;

  writer.write_u8(controller_data.analog_d_pad_left)?;
  writer.write_u8(controller_data.analog_d_pad_down)?;
  writer.write_u8(controller_data.analog_d_pad_right)?;
  writer.write_u8(controller_data.analog_d_pad_up)?;

  writer.write_u8(controller_data.analog_square)?;
  writer.write_u8(controller_data.analog_cross)?;
  writer.write_u8(controller_data.analog_circle)?;
  writer.write_u8(controller_data.analog_triangle)?;

  writer.write_u8(controller_data.analog_r1)?;
  writer.write_u8(controller_data.analog_l1)?;
  writer.write_u8(controller_data.analog_r2)?;
  writer.write_u8(controller_data.analog_l2)?;

  encode_touch_data(writer, controller_data.first_touch)?;
  encode_touch_data(writer, controller_data.second_touch)?;

  writer.write_u64::<LittleEndian>(controller_data.motion_data_timestamp)?;

  writer.write_f32::<LittleEndian>(controller_data.accelerometer_x)?;
  writer.write_f32::<LittleEndian>(controller_data.accelerometer_y)?;
  writer.write_f32::<LittleEndian>(controller_data.accelerometer_z)?;

  writer.write_f32::<LittleEndian>(controller_data.gyroscope_pitch)?;
  writer.write_f32::<LittleEndian>(controller_data.gyroscope_yaw)?;
  writer.write_f32::<LittleEndian>(controller_data.gyroscope_roll)
}

pub fn parse_controller_data(reader: &mut Cursor<&[u8]>) -> Result<(u32, ControllerData)> {
  let connected = {
    let connected = reader.read_u8()?;

    match connected {
      0 => Ok(false),
      1 => Ok(true),
      _ => Err(invalid_data_error("Invalid connected value"))
    }
  }?;
  
  let packet_number = reader.read_u32::<LittleEndian>()?;

  let button_data = reader.read_u8()?;
  let d_pad_left =         (button_data & 0b10000000) != 0;
  let d_pad_down =         (button_data & 0b01000000) != 0;
  let d_pad_right =        (button_data & 0b00100000) != 0;
  let d_pad_up =           (button_data & 0b00010000) != 0;
  let start =              (button_data & 0b00001000) != 0;
  let right_stick_button = (button_data & 0b00000100) != 0;
  let left_stick_button =  (button_data & 0b00000010) != 0;
  let select =             (button_data & 0b00000001) != 0;

  let button_data = reader.read_u8()?;
  let square =             (button_data & 0b10000000) != 0;
  let cross =              (button_data & 0b01000000) != 0;
  let circle =             (button_data & 0b00100000) != 0;
  let triangle =           (button_data & 0b00010000) != 0;
  let r1 =                 (button_data & 0b00001000) != 0;
  let l1 =                 (button_data & 0b00000100) != 0;
  let r2 =                 (button_data & 0b00000010) != 0;
  let l2 =                 (button_data & 0b00000001) != 0;

  let ps = reader.read_u8()?;

  let touch = reader.read_u8()?;

  let left_stick_x = reader.read_u8()?;
  let left_stick_y = reader.read_u8()?;

  let right_stick_x = reader.read_u8()?;
  let right_stick_y = reader.read_u8()?;

  let analog_d_pad_left = reader.read_u8()?;
  let analog_d_pad_down = reader.read_u8()?;
  let analog_d_pad_right = reader.read_u8()?;
  let analog_d_pad_up = reader.read_u8()?;

  let analog_square = reader.read_u8()?;
  let analog_cross = reader.read_u8()?;
  let analog_circle = reader.read_u8()?;
  let analog_triangle = reader.read_u8()?;

  let analog_r1 = reader.read_u8()?;
  let analog_l1 = reader.read_u8()?;
  let analog_r2 = reader.read_u8()?;
  let analog_l2 = reader.read_u8()?;

  let first_touch = parse_touch_data(reader)?;
  let second_touch = parse_touch_data(reader)?;

  let motion_data_timestamp = reader.read_u64::<LittleEndian>()?;

  let accelerometer_x = reader.read_f32::<LittleEndian>()?;
  let accelerometer_y = reader.read_f32::<LittleEndian>()?;
  let accelerometer_z = reader.read_f32::<LittleEndian>()?;

  let gyroscope_pitch = reader.read_f32::<LittleEndian>()?;
  let gyroscope_yaw = reader.read_f32::<LittleEndian>()?;
  let gyroscope_roll = reader.read_f32::<LittleEndian>()?;

  Ok((packet_number, ControllerData {
    connected,
    d_pad_left,
    d_pad_down,
    d_pad_right,
    d_pad_up,
    start,
    right_stick_button,
    left_stick_button,
    select,
    square,
    cross,
    circle,
    triangle,
    r1,
    l1,
    r2,
    l2,
    ps,
    touch,
    left_stick_x,
    left_stick_y,
    right_stick_x,
    right_stick_y,
    analog_d_pad_left,
    analog_d_pad_down,
    analog_d_pad_right,
    analog_d_pad_up,
    analog_square,
    analog_triangle,
    analog_circle,
    analog_cross,
    analog_r1,
    analog_l1,
    analog_r2,
    analog_l2,
    first_touch,
    second_touch,
    motion_data_timestamp,
    accelerometer_x,
    accelerometer_y,
    accelerometer_z,
    gyroscope_pitch,
    gyroscope_yaw,
    gyroscope_roll
  }))
}

pub fn encode_message_payload(writer: &mut Vec<u8>, 
                              message_payload: MessagePayload) -> Result<()> {
  match message_payload {
    MessagePayload::None => Ok(()),
    MessagePayload::ProtocolVersion(version) => {
      writer.write_u16::<LittleEndian>(version)
    },
    MessagePayload::ConnectedControllersRequest { amount, slot_numbers } => {
      if amount < 0 || amount > 4 {
        return Err(invalid_data_error("Invalid amount of ports to report"))
      };

      writer.write_i32::<LittleEndian>(amount)?;

      for i in 0..amount {
        writer.write_u8(slot_numbers[i as usize])?;
      }

      Ok(())
    },
    MessagePayload::ConnectedControllerResponse { controller_info } => {
      encode_controller_info(writer, controller_info)?;
      writer.write_u8(0)
    },
    MessagePayload::ControllerDataRequest(request) => {
      encode_controller_data_request(writer, request)
    },
    MessagePayload::ControllerData { packet_number,
                                     controller_info,
                                     controller_data } => {
      encode_controller_info(writer, controller_info)?;
      encode_controller_data(writer, packet_number, controller_data)
    }
  }
}

pub fn parse_message_payload(reader: &mut Cursor<&[u8]>,
                             message_source: MessageSource,
                             message_type: MessageType) -> Result<MessagePayload> {
  match message_source {
    MessageSource::Server => {
      match message_type {
        MessageType::ProtocolVersion => {
          let protocol_version = reader.read_u16::<LittleEndian>()?;
          Ok(MessagePayload::ProtocolVersion(protocol_version))
        },
        MessageType::ConnectedControllers => {
          let controller_info = parse_controller_info(reader)?;
          let terminating_byte = reader.read_u8()?;
          if terminating_byte != 0 {
            Err(invalid_data_error("Message was not properly terminated"))
          } else {
            Ok(MessagePayload::ConnectedControllerResponse {
              controller_info
            })
          }
        },
        MessageType::ControllerData => {
          let controller_info = parse_controller_info(reader)?;
          let (packet_number, controller_data) = parse_controller_data(reader)?;
          
          Ok(MessagePayload::ControllerData {
            packet_number,
            controller_info,
            controller_data
          })
        }
      }
    },
    MessageSource::Client => {
      match message_type {
        MessageType::ProtocolVersion => {
          Ok(MessagePayload::None)
        }
        MessageType::ConnectedControllers => {
          let amount = reader.read_i32::<LittleEndian>()?;
          if amount < 0 || amount > 4 {
            return Err(invalid_data_error("Invalid amount of ports to report"))
          };

          let mut slot_numbers = [0; 4];
          for i in 0..amount {
            let slot_number = reader.read_u8()?;
            if slot_number >= 4 {
              return Err(invalid_data_error("Invalid slot number"))
            }

            slot_numbers[i as usize] = slot_number;
          }

          Ok(MessagePayload::ConnectedControllersRequest {
            amount,
            slot_numbers
          })
        }
        MessageType::ControllerData => {
          let controller_data_request = parse_controller_data_request(reader)?;

          Ok(MessagePayload::ControllerDataRequest(controller_data_request))
        }
      }
    }
  }
}
