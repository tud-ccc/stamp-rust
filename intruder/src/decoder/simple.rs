use super::DecodedPacket;
use crate::Packet;
use std::collections::HashMap;

pub struct DecoderState {
    pub fragments_map: HashMap<usize, Vec<Packet>>,
}

impl DecoderState {
    pub fn new() -> Self {
        Self {
            fragments_map: HashMap::new(),
        }
    }
}

/// Decodes a packet. If the packet is part of a fragmented flow and the flow is
/// not completely decoded yet, the function will return `None` and place the
/// packet in its internal storage
pub fn decode_packet(packet: Packet, state: &mut DecoderState) -> Option<DecodedPacket> {
    if packet.packets_in_flow != 1 {
        // this is part of a fragmented flow
        let decoded = state
            .fragments_map
            .entry(packet.flow_id)
            .or_insert(Vec::new());

        // insert the current element into the queue
        let idx = decoded
            .iter()
            .position(|p| packet.fragment_id < p.fragment_id)
            .unwrap_or(decoded.len());
        decoded.insert(idx, packet);

        // reassemble the flow if all fragments are present
        if decoded.len() == decoded[0].packets_in_flow {
            let flow_id = decoded[0].flow_id;
            let reconstructed_data = decoded
                .drain(..)
                .fold(String::new(), |acc, p| acc + &p.data);

            // TODO: Remove assertion?
            assert!(state.fragments_map.remove(&flow_id).is_some());
            Some(DecodedPacket {
                flow_id,
                data: reconstructed_data,
            })
        } else {
            None
        }
    } else {
        // this is the only packet
        assert!(packet.fragment_id == 0);

        Some(DecodedPacket {
            flow_id: packet.flow_id,
            data: packet.data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Packet;

    #[test]
    fn basic_decoding() {
        let inp1 = Packet {
            flow_id: 42,
            fragment_id: 1,
            packets_in_flow: 3,
            length: 1,
            data: "w".into(),
        };
        let inp2 = Packet {
            flow_id: 42,
            fragment_id: 0,
            packets_in_flow: 3,
            length: 1,
            data: "t".into(),
        };
        let inp3 = Packet {
            flow_id: 42,
            fragment_id: 2,
            packets_in_flow: 3,
            length: 1,
            data: "o".into(),
        };

        let mut dec = DecoderState::new();

        assert_eq!(decode_packet(inp1, &mut dec), None);
        assert_eq!(decode_packet(inp2, &mut dec), None);
        assert_eq!(
            decode_packet(inp3, &mut dec),
            Some(DecodedPacket {
                flow_id: 42,
                data: "two".into()
            })
        );
    }
}
