use std::io;

#[derive(Debug)]
pub enum UdpSendError {
    Oversized {
        encoded_len: usize,
        max_packet_bytes: usize,
    },
    Io(io::Error),
}

impl core::fmt::Display for UdpSendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Oversized {
                encoded_len,
                max_packet_bytes,
            } => write!(
                f,
                "datagram {encoded_len} bytes exceeds max_packet_bytes {max_packet_bytes}"
            ),
            Self::Io(e) => write!(f, "udp send: {e}"),
        }
    }
}

impl std::error::Error for UdpSendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Oversized { .. } => None,
            Self::Io(e) => Some(e),
        }
    }
}
