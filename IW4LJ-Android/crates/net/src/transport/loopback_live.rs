use bevy::prelude::Resource;
use sim::{ClientId, Snapshot};

use crate::client::predict::CmdSeq;
use crate::transport::delta::{SnapshotDecoder, SnapshotEncoder};
use crate::transport::frame::{Frame, TransportError, frame_for_local_reader};

#[derive(Resource, Debug, Default)]
pub struct ListenLoopback {
    /// Frames waiting for the client half, which lives in this process: they
    /// are passed as they are, with no trip through the wire format.
    queue: std::collections::VecDeque<Frame>,
    encoder: SnapshotEncoder,
    decoder: SnapshotDecoder,

    pub sent: u64,

    pub received: u64,

    pub last_backlog: usize,

    pub peak_backlog: usize,
}

impl ListenLoopback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_tick(
        &mut self,
        snapshot: &Snapshot,
        acks: Vec<(ClientId, CmdSeq)>,
        svc_sounds: Vec<crate::SvcSound>,
        svc_scores: Option<String>,
        svc_card_slots: Vec<crate::SvcCardSlot>,
        svc_open_menus: Vec<crate::SvcOpenMenu>,
        svc_hud_splashes: Vec<crate::SvcHudSplash>,
        svc_game_notifies: Vec<crate::SvcGameNotify>,
        reliable: crate::ReliablePayload,
    ) -> Result<(), TransportError> {
        let mut frame = frame_for_local_reader(
            &mut self.encoder,
            snapshot,
            acks,
            crate::ReliablePayload::owes_nothing(),
        );
        frame.svc_sounds = svc_sounds;
        frame.svc_scores = svc_scores;
        frame.svc_card_slots = svc_card_slots;
        frame.svc_open_menus = svc_open_menus;
        frame.svc_hud_splashes = svc_hud_splashes;
        frame.svc_game_notifies = svc_game_notifies;

        frame.reliable = reliable;
        self.queue.push_back(frame);
        self.sent = self.sent.saturating_add(1);
        Ok(())
    }

    pub fn recv_tick(&mut self) -> Result<Option<ReceivedTick>, TransportError> {
        let Some(mut frame) = self.queue.pop_front() else {
            return Ok(None);
        };
        let mut snapshot = self.decoder.decode(&frame.snapshot_delta)?;
        snapshot.meta = std::mem::take(&mut frame.snapshot_meta);
        self.received = self.received.saturating_add(1);
        Ok(Some(ReceivedTick { snapshot, frame }))
    }

    pub fn recv_all(&mut self, out: &mut Vec<ReceivedTick>) -> Result<(), TransportError> {
        self.last_backlog = self.queue.len();
        self.peak_backlog = self.peak_backlog.max(self.last_backlog);
        loop {
            match self.recv_tick() {
                Ok(Some(tick)) => out.push(tick),
                Ok(None) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Debug)]
pub struct ReceivedTick {
    pub snapshot: Snapshot,
    pub frame: Frame,
}

impl ReceivedTick {
    pub fn ack_for(&self, client: ClientId) -> Option<CmdSeq> {
        self.frame.ack_for(client)
    }
}
