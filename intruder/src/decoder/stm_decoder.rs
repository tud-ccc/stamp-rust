use super::DecodedPacket;
use crate::Packet;
use stm::{StmError, StmResult, Transaction, TVar};
use stm_datastructures::THashMap;
use std::collections::HashMap;

#[derive(Clone)]
pub struct StmDecoderState {
    pub fragments_map: THashMap<usize, TVar<Vec<Packet>>>,
}

impl StmDecoderState {
    pub fn new(bucket_no: usize) -> Self {
        Self {
            fragments_map: THashMap::new(bucket_no),
        }
    }
}

/// Decodes a packet. If the packet is part of a fragmented flow and the flow is
/// not completely decoded yet, the function will return `None` and place the
/// packet in its internal storage
pub fn decode_packet(
    packet: &Packet,
    state: &StmDecoderState,
    transaction: &mut Transaction,
) -> StmResult<Option<DecodedPacket>> {
    if packet.packets_in_flow != 1 {
        // get the matching TVar (== bucket) and read it
        let bucket = state.fragments_map.get_bucket(&packet.flow_id);
        let frags = bucket.read_ref_atomic().downcast::<HashMap<usize, TVar<Vec<Packet>>>>().unwrap();

        if let Some(decoded_tv) = frags.get(&packet.flow_id) {
            // we already have some packets with that ID

            let mut decoded = decoded_tv.read(transaction)?;
            // insert the current element into the queue
            let idx = decoded
                .iter()
                .position(|p| packet.fragment_id < p.fragment_id)
                .unwrap_or(decoded.len());
            decoded.insert(idx, packet.to_owned());

            // reassemble the flow if all fragments are present
            if decoded.len() == decoded[0].packets_in_flow {
                let flow_id = decoded[0].flow_id;
                let reconstructed_data = decoded
                    .into_iter()
                    .fold(String::new(), |acc, p| acc + &p.data);

                // remove the flow from the hashmap & write that back
                bucket.modify(transaction, |mut hm| { let _ = hm.remove(&flow_id); hm })?;

                Ok(Some(DecodedPacket {
                    flow_id,
                    data: reconstructed_data,
                }))
            } else {
                decoded_tv.write(transaction, decoded)?;
                Ok(None)
            }
        } else {
            // This is the first Item in the flow we see
            let flow_id = packet.flow_id;
            let mut v = Vec::with_capacity(packet.packets_in_flow);
            v.push(packet.to_owned());
            let state = TVar::new(v);

            let mut hm = bucket.read(transaction)?;
            // must do this test since we did an atomic read before and with many threads it can
            // happen that we construct data races :facepalm:
            if hm.contains_key(&flow_id) {
                return Err(StmError::Retry);
            }

            hm.insert(flow_id, state);
            bucket.write(transaction, hm)?;

            // it can by definition never happen that this branch will complete a flow, so that's
            // it for this one
            Ok(None)
        }
    } else {
        // this is the only packet
        assert!(packet.fragment_id == 0);

        Ok(Some(DecodedPacket {
            flow_id: packet.flow_id,
            data: packet.data.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Packet;
    use stm::atomically;

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

        let dec = StmDecoderState::new();

        assert_eq!(atomically(|trans| decode_packet(&inp1, &dec, trans)), None);
        assert_eq!(atomically(|trans| decode_packet(&inp2, &dec, trans)), None);
        assert_eq!(
            atomically(|trans| decode_packet(&inp3, &dec, trans)),
            Some(DecodedPacket {
                flow_id: 42,
                data: "two".into()
            })
        );
    }
}
