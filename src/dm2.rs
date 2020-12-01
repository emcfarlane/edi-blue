// EEPROM
const XL_MODEL_NUMBER_L: u8 = 0;
const XL_MODEL_NUMBER_H: u8 = 1;
const XL_VERSION: u8 = 2;
const XL_ID: u8 = 3;
const XL_BAUD_RATE: u8 = 4;
const XL_RETURN_DELAY_TIME: u8 = 5;
const XL_CW_ANGLE_LIMIT_L: u8 = 6;
const XL_CW_ANGLE_LIMIT_H: u8 = 7;
const XL_CCW_ANGLE_LIMIT_L: u8 = 8;
const XL_CCW_ANGLE_LIMIT_H: u8 = 9;
const XL_CONTROL_MODE: u8 = 11;
const XL_LIMIT_TEMPERATURE: u8 = 12;
const XL_DOWN_LIMIT_VOLTAGE: u8 = 13;
const XL_UP_LIMIT_VOLTAGE: u8 = 14;
const XL_MAX_TORQUE_L: u8 = 15;
const XL_MAX_TORQUE_H: u8 = 16;
const XL_RETURN_LEVEL: u8 = 17;
const XL_ALARM_SHUTDOWN: u8 = 18;
// RAM
const XL_TORQUE_ENABLE: u8 = 24;
const XL_LED: u8 = 25;
const XL_D_GAIN: u8 = 27;
const XL_I_GAIN: u8 = 28;
const XL_P_GAIN: u8 = 29;
const XL_GOAL_POSITION_L: u8 = 30;
const XL_GOAL_SPEED_L: u8 = 32;
const XL_GOAL_TORQUE: u8 = 35;
const XL_PRESENT_POSITION: u8 = 37;
const XL_PRESENT_SPEED: u8 = 39;
const XL_PRESENT_LOAD: u8 = 41;
const XL_PRESENT_VOLTAGE: u8 = 45;
const XL_PRESENT_TEMPERATURE: u8 = 46;
const XL_REGISTERED_INSTRUCTION: u8 = 47;
const XL_MOVING: u8 = 49;
const XL_HARDWARE_ERROR: u8 = 50;
const XL_PUNCH: u8 = 51;
// INS
const INS_Ping: u8 = 0x01; // Corresponding device ID command to check if packet reaches
const INS_Read: u8 = 0x02; // Read command
const INS_Write: u8 = 0x03; // Write command
const INS_RegWrite: u8 = 0x04; // When receiving a write command packet data is not immediately written instead it goes into standby momentarily until action command arrives
const INS_Action: u8 = 0x05; // Go command for Reg Write
const INS_Factory: u8 = 0x06; // Reset All data to factory default settings
const INS_Reboot: u8 = 0x08; // Reboot device
const INS_StatusReturn: u8 = 0x55; // Instruction Packet response
const INS_SyncRead: u8 = 0x82; // Read data from the same location and same size for multiple devices simultaneously
const INS_SyncWrite: u8 = 0x83; // Write data from the same location and same size for multiple devices simultaneously
const INS_BulkRead: u8 = 0x92; // Read data from the different locations and different sizes for multiple devices simultaneously
const INS_BulkWrite: u8 = 0x93; // Write data from the different locations and different sizes for multiple devices simultaneously
                                // ID
pub const ID_Broadcast: u8 = 0xFE; // 254(0xFE) is used as the Broadcast ID

// HELP
// http://support.robotis.com/en/product/actuator/dynamixel_pro/communication/instruction_status_packet.htm
//

//#define DM_MAKEWORD(a, b) ((unsigned short)(((unsigned char)(((unsigned long)(a)) & 0xff)) | ((unsigned short)((unsigned char)(((unsigned long)(b)) & 0xff))) << 8))
//#define DM_LOBYTE(w) ((unsigned char)(((unsigned long)(w)) & 0xff))
//#define DM_HIBYTE(w) ((unsigned char)((((unsigned long)(w)) >> 8) & 0xff))
//
fn DM_LOBYTE(x: u16) -> u8 {
    x as u8
}
fn DM_HIBYTE(x: u16) -> u8 {
    (x >> 8) as u8
}
fn DM_MAKEWORD(a: u8, b: u8) -> u16 {
    (a as u16) | ((b as u16) << 8)
}

//unsigned short update_crc(unsigned short crc_accum, unsigned char *data_blk_ptr, unsigned short data_blk_size) {
fn update_crc(mut crc_accum: u16, data: &[u8]) -> u16 {
    let crc_table: [u16; 256] = [
        0x0000, 0x8005, 0x800F, 0x000A, 0x801B, 0x001E, 0x0014, 0x8011, 0x8033, 0x0036, 0x003C,
        0x8039, 0x0028, 0x802D, 0x8027, 0x0022, 0x8063, 0x0066, 0x006C, 0x8069, 0x0078, 0x807D,
        0x8077, 0x0072, 0x0050, 0x8055, 0x805F, 0x005A, 0x804B, 0x004E, 0x0044, 0x8041, 0x80C3,
        0x00C6, 0x00CC, 0x80C9, 0x00D8, 0x80DD, 0x80D7, 0x00D2, 0x00F0, 0x80F5, 0x80FF, 0x00FA,
        0x80EB, 0x00EE, 0x00E4, 0x80E1, 0x00A0, 0x80A5, 0x80AF, 0x00AA, 0x80BB, 0x00BE, 0x00B4,
        0x80B1, 0x8093, 0x0096, 0x009C, 0x8099, 0x0088, 0x808D, 0x8087, 0x0082, 0x8183, 0x0186,
        0x018C, 0x8189, 0x0198, 0x819D, 0x8197, 0x0192, 0x01B0, 0x81B5, 0x81BF, 0x01BA, 0x81AB,
        0x01AE, 0x01A4, 0x81A1, 0x01E0, 0x81E5, 0x81EF, 0x01EA, 0x81FB, 0x01FE, 0x01F4, 0x81F1,
        0x81D3, 0x01D6, 0x01DC, 0x81D9, 0x01C8, 0x81CD, 0x81C7, 0x01C2, 0x0140, 0x8145, 0x814F,
        0x014A, 0x815B, 0x015E, 0x0154, 0x8151, 0x8173, 0x0176, 0x017C, 0x8179, 0x0168, 0x816D,
        0x8167, 0x0162, 0x8123, 0x0126, 0x012C, 0x8129, 0x0138, 0x813D, 0x8137, 0x0132, 0x0110,
        0x8115, 0x811F, 0x011A, 0x810B, 0x010E, 0x0104, 0x8101, 0x8303, 0x0306, 0x030C, 0x8309,
        0x0318, 0x831D, 0x8317, 0x0312, 0x0330, 0x8335, 0x833F, 0x033A, 0x832B, 0x032E, 0x0324,
        0x8321, 0x0360, 0x8365, 0x836F, 0x036A, 0x837B, 0x037E, 0x0374, 0x8371, 0x8353, 0x0356,
        0x035C, 0x8359, 0x0348, 0x834D, 0x8347, 0x0342, 0x03C0, 0x83C5, 0x83CF, 0x03CA, 0x83DB,
        0x03DE, 0x03D4, 0x83D1, 0x83F3, 0x03F6, 0x03FC, 0x83F9, 0x03E8, 0x83ED, 0x83E7, 0x03E2,
        0x83A3, 0x03A6, 0x03AC, 0x83A9, 0x03B8, 0x83BD, 0x83B7, 0x03B2, 0x0390, 0x8395, 0x839F,
        0x039A, 0x838B, 0x038E, 0x0384, 0x8381, 0x0280, 0x8285, 0x828F, 0x028A, 0x829B, 0x029E,
        0x0294, 0x8291, 0x82B3, 0x02B6, 0x02BC, 0x82B9, 0x02A8, 0x82AD, 0x82A7, 0x02A2, 0x82E3,
        0x02E6, 0x02EC, 0x82E9, 0x02F8, 0x82FD, 0x82F7, 0x02F2, 0x02D0, 0x82D5, 0x82DF, 0x02DA,
        0x82CB, 0x02CE, 0x02C4, 0x82C1, 0x8243, 0x0246, 0x024C, 0x8249, 0x0258, 0x825D, 0x8257,
        0x0252, 0x0270, 0x8275, 0x827F, 0x027A, 0x826B, 0x026E, 0x0264, 0x8261, 0x0220, 0x8225,
        0x822F, 0x022A, 0x823B, 0x023E, 0x0234, 0x8231, 0x8213, 0x0216, 0x021C, 0x8219, 0x0208,
        0x820D, 0x8207, 0x0202,
    ];

    for element in data {
        let i = DM_HIBYTE(crc_accum) ^ element;
        crc_accum = (crc_accum << 8) ^ crc_table[i as usize];
    }
    return crc_accum;
}

// FF, FF, FD, 0, 1, 7, 0, 2, 0, 0, 0, 2, 85, B7
fn test_packet() -> [u8; 14] {
    let mut data: [u8; 14] = [
        0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
    ];

    let crc = update_crc(0, &data[0..12]);
    data[12] = DM_LOBYTE(crc);
    data[13] = DM_HIBYTE(crc);
    println!("CRC: {} {}", data[12], data[13]);
    data
}

#[derive(Debug)]
struct Command {
    address: u16,
    value: u16,
}

// 0   1   2   3  4   5  6  7  8   9  10 11 12  13
// FF, FF, FD, 0, FE, 7, 0, 3, 19, 0, 2, 0, D4, 3D
fn dataPacket(id: u8, buf: &mut [u8], ins: u8, cmds: &[Command]) -> usize {
    // Header
    buf[0] = 0xFF;
    buf[1] = 0xFF;
    buf[2] = 0xFD;

    // Reserved
    buf[3] = 0x00;

    // ID
    buf[4] = id;

    // Packet Length
    let bytes = cmds.len() * 4;
    buf[5] = DM_LOBYTE((bytes + 3) as u16);
    buf[6] = DM_HIBYTE((bytes + 3) as u16);

    // Instruction
    buf[7] = ins;

    let mut i: usize = 0;
    for cmd in cmds.iter() {
        dataPack(&mut buf[8 + i..8 + i + 4], cmd.address, cmd.value);
        i += 4;
    }

    let crc = update_crc(0, &buf[0..bytes + 8]);
    buf[bytes + 8] = DM_LOBYTE(crc);
    buf[bytes + 9] = DM_HIBYTE(crc);

    return bytes + 10;
}

// dataPack sets data in an array.
fn dataPack(buf: &mut [u8], address: u16, value: u16) -> usize {
    buf[0] = DM_LOBYTE(address);
    buf[1] = DM_HIBYTE(address);
    buf[2] = DM_LOBYTE(value);
    buf[3] = DM_HIBYTE(value);
    return 4;
}

fn dataPush(id: u8, cmd: u8, val: u16) -> [u8; 14] {
    let mut buf: [u8; 14] = [0; 14];
    let cmds: [Command; 1] = [Command {
        address: cmd as u16,
        value: val,
    }];
    dataPacket(id, &mut buf, INS_Write, &cmds);
    return buf;
}

//// SetLED sets motor led colours.
//// r = 1, g = 2, y = 3, b = 4, p = 5, c = 6, w = 7, o = 0
//int DM2::SetLED(int ID, int colour){
//	return dataPush(ID, XL_LED, colour);
//}
//
pub fn set_led(id: u8, colour: u16) -> [u8; 14] {
    dataPush(id, XL_LED, colour)
}

// SetJointMode
// 1 = Wheel Mode, 2 = Joint Mode
pub fn set_joint_mode(id: u8, mut value: u16) -> [u8; 14] {
    if value != 1 || value != 2 {
        value = 1;
    }
    println!("SET_JOINT_MODE: {}", value);

    dataPush(id, XL_CONTROL_MODE, value)
}

// SetAngleLimit
// CW = 0, CCW = 1023
pub fn set_angle_limit(id: u8, cw: u16, ccw: u16) -> [u8; 18] {
    println!("SET_JOINT_LIMITS: CW({}) CCW({})", cw, ccw);

    let mut buf: [u8; 18] = [0; 18];
    let cmds: [Command; 2] = [
        Command {
            address: XL_CW_ANGLE_LIMIT_L as u16,
            value: cw,
        },
        Command {
            address: XL_CCW_ANGLE_LIMIT_L as u16,
            value: ccw,
        },
    ];
    dataPacket(id, &mut buf, INS_Write, &cmds);
    return buf;
}

// SetSpeed
// https://emanual.robotis.com/docs/en/dxl/x/xl320/#moving-speed
pub fn set_speed(id: u8, mut angle: f32) -> [u8; 14] {
    let value: u16 = if angle >= 0.0 {
        if angle > 1.0 {
            angle = 1.0;
        };
        (angle * 1023.0) as u16
    } else {
        if angle < -1.0 {
            angle = -1.0
        };
        (angle * -1023.0 + 1024.0) as u16
    };
    println!("SET_SPEED: {}", value);

    dataPush(id, XL_GOAL_SPEED_L, value)
}

pub fn set_pos(id: u8, mut angle: f32) -> [u8; 14] {
    if angle < 0.0 {
        angle = angle * -1.0;
    };
    if angle > 1.0 {
        angle = 1.0;
    };
    let value: u16 = (angle * 1024.0) as u16;
    println!("SET_POS: {}", value);

    dataPush(id, XL_GOAL_POSITION_L, value)
}

pub fn set_torque(id: u8, value: u16) -> [u8; 14] {
    dataPush(id, XL_TORQUE_ENABLE, value)
}

pub fn ping(id: u8) -> [u8; 10] {
    let mut buf: [u8; 10] = [0; 10];
    let cmds: [Command; 0] = [];
    dataPacket(id, &mut buf, INS_Ping, &cmds);
    return buf;
}
