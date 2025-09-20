use std::{fmt::Debug, fmt::Display};

// Info packet structure:
// [0xff, target, 0x10, pageSize, nBuffPage, nFlashPage, flashStart, cpuId, version]
//
// Command: 0x10
// pageSize (2 bytes): Size of flash and buffer pages
// nBuffPage (2 bytes): Number of RAM buffer pages available
// nFlashPage (2 bytes): Total number of flash pages
// flashStart (2 bytes): Start flash page of firmware
// cpuId (12 bytes): Legacy CPU ID (should be ignored)
// version (1 byte): Protocol version
pub struct InfoPacket {
    page_size: u16,
    n_buff_page: u16,
    n_flash_page: u16,
    flash_start: u16,
    cpu_id: [u8; 12],
    version: u8,
}

impl InfoPacket {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 22 {
            panic!("Invalid InfoPacket length: expected at least 22 bytes, got {}", bytes.len());
        }
        InfoPacket {
            page_size: u16::from_le_bytes([bytes[1], bytes[2]]),
            n_buff_page: u16::from_le_bytes([bytes[3], bytes[4]]),
            n_flash_page: u16::from_le_bytes([bytes[5], bytes[6]]),
            flash_start: u16::from_le_bytes([bytes[7], bytes[8]]),
            cpu_id: [bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16], bytes[17], bytes[18], bytes[19], bytes[20]],
            version: bytes[21],
        }
    }

    pub fn page_size(&self) -> u16 {
        self.page_size
    }

    pub fn n_buff_page(&self) -> u16 {
        self.n_buff_page
    }

    pub fn n_flash_page(&self) -> u16 {
        self.n_flash_page
    }

    pub fn flash_start(&self) -> u16 {
        self.flash_start
    }

    pub fn version(&self) -> u8 {
        self.version
    }
}

impl Debug for InfoPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("InfoPacket")
            .field("page_size", &self.page_size)
            .field("n_buff_page", &self.n_buff_page)
            .field("n_flash_page", &self.n_flash_page)
            .field("flash_start", &self.flash_start)
            .field("cpu_id", &self.cpu_id)
            .field("version", &self.version)
            .finish()
    }
}

impl Display for InfoPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "InfoPacket {{ page_size: {}, n_buff_page: {}, n_flash_page: {}, flash_start: {}, cpu_id: {:?}, version: {} }}",
               self.page_size, self.n_buff_page, self.n_flash_page, self.flash_start, self.cpu_id, self.version)
    }
}

// Buffer read packet structure
pub struct BufferReadPacket {
    pub page: u16,
    pub address: u16,
    pub data: Vec<u8>,
}

impl BufferReadPacket {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 5 {
            panic!("Invalid BufferReadPacket length");
        }
        BufferReadPacket {
            page: u16::from_le_bytes([bytes[1], bytes[2]]),
            address: u16::from_le_bytes([bytes[3], bytes[4]]),
            data: bytes[5..].to_vec(),
        }
    }
}

impl Debug for BufferReadPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("BufferReadPacket")
            .field("page", &self.page)
            .field("address", &self.address)
            .field("data", &self.data)
            .finish()
    }
}

// Flash write response structure
pub struct FlashWriteResponse {
    pub done: u8,
    pub error: u8,
}

impl FlashWriteResponse {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 3 {
            panic!("Invalid FlashWriteResponse length");
        }
        FlashWriteResponse {
            done: bytes[1],
            error: bytes[2],
        }
    }

    pub fn is_done(&self) -> bool {
        self.done != 0
    }

    pub fn error(&self) -> FlashError {
        FlashError::from(self.error)
    }

    pub fn is_success(&self) -> bool {
        self.is_done() && self.error() == FlashError::NoError
    }
}

impl Debug for FlashWriteResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("FlashWriteResponse")
            .field("done", &self.done)
            .field("error", &self.error)
            .finish()
    }
}

// Flash status response structure (same as FlashWriteResponse)
pub type FlashStatusResponse = FlashWriteResponse;

// Flash read packet structure
pub struct FlashReadPacket {
    pub page: u16,
    pub address: u16,
    pub data: Vec<u8>,
}

impl FlashReadPacket {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() < 5 {
            panic!("Invalid FlashReadPacket length");
        }
        FlashReadPacket {
            page: u16::from_le_bytes([bytes[1], bytes[2]]),
            address: u16::from_le_bytes([bytes[3], bytes[4]]),
            data: bytes[5..].to_vec(),
        }
    }
}

impl Debug for FlashReadPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("FlashReadPacket")
            .field("page", &self.page)
            .field("address", &self.address)
            .field("data", &self.data)
            .finish()
    }
}

// Error codes enum for flash operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlashError {
    NoError = 0,
    AddressOutOfBounds = 1,
    FlashEraseFailed = 2,
    FlashProgrammingFailed = 3,
}

impl From<u8> for FlashError {
    fn from(value: u8) -> Self {
        match value {
            0 => FlashError::NoError,
            1 => FlashError::AddressOutOfBounds,
            2 => FlashError::FlashEraseFailed,
            3 => FlashError::FlashProgrammingFailed,
            _ => FlashError::NoError, // Default to no error for unknown codes
        }
    }
}

impl Display for FlashError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FlashError::NoError => write!(f, "No error"),
            FlashError::AddressOutOfBounds => write!(f, "Addresses are outside of authorized boundaries"),
            FlashError::FlashEraseFailed => write!(f, "Flash erase failed"),
            FlashError::FlashProgrammingFailed => write!(f, "Flash programming failed"),
        }
    }
}