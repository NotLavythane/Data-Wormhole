//! Six-state transfer UI model
//! 
//! States:
//! - Scanning: Looking for QR codes
//! - KeyExchange: Verifying safety number
//! - Receiving: Actively receiving data
//! - Reconstructing: Reassembling fountain blocks
//! - Decrypting: Verifying integrity
//! - Complete: Transfer done
//! - Stalled: Recovery needed

use crate::ProtocolError;

/// Transfer states for UI and protocol state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Idle = 0,
    Scanning = 1,
    KeyExchange = 2,
    Receiving = 3,
    Reconstructing = 4,
    Decrypting = 5,
    Complete = 6,
    Stalled = 7,
}

impl TransferState {
    /// Check if transition is valid
    pub fn can_transition_to(&self, next: TransferState) -> bool {
        use TransferState::*;
        
        match (*self, next) {
            // From Idle
            (Idle, Scanning) => true,
            
            // From Scanning
            (Scanning, KeyExchange) => true,
            (Scanning, Stalled) => true,
            
            // From KeyExchange
            (KeyExchange, Receiving) => true,
            (KeyExchange, Stalled) => true,
            
            // From Receiving
            (Receiving, Reconstructing) => true,
            (Receiving, Stalled) => true,
            (Receiving, KeyExchange) => true, // For re-key
            
            // From Reconstructing
            (Reconstructing, Decrypting) => true,
            (Reconstructing, Receiving) => true, // Need more blocks
            
            // From Decrypting
            (Decrypting, Complete) => true,
            (Decrypting, Receiving) => true, // Missing chunks
            
            // From Stalled
            (Stalled, Scanning) => true,
            (Stalled, Receiving) => true,
            (Stalled, KeyExchange) => true,
            
            // From Complete
            (Complete, Idle) => true,
            
            // Same state is always valid
            (a, b) if a == b => true,
            
            _ => false,
        }
    }
    
    /// Get human-readable state name
    pub fn display_name(&self) -> &'static str {
        match self {
            TransferState::Idle => "Idle",
            TransferState::Scanning => "Scanning",
            TransferState::KeyExchange => "Key Exchange",
            TransferState::Receiving => "Receiving",
            TransferState::Reconstructing => "Reconstructing",
            TransferState::Decrypting => "Decrypting",
            TransferState::Complete => "Complete",
            TransferState::Stalled => "Stalled",
        }
    }
    
    /// Get UI message for this state
    pub fn ui_message(&self) -> &'static str {
        match self {
            TransferState::Idle => "Ready to transfer",
            TransferState::Scanning => "Point camera at sender QR code",
            TransferState::KeyExchange => "Verify the pattern matches on both devices",
            TransferState::Receiving => "Receiving encrypted file",
            TransferState::Reconstructing => "Assembling file",
            TransferState::Decrypting => "Verifying integrity",
            TransferState::Complete => "Transfer complete",
            TransferState::Stalled => "No QR detected. Try: move closer, increase brightness, clean lens",
        }
    }
}

/// State machine for managing transfer lifecycle
pub struct TransferStateMachine {
    current: TransferState,
    previous: Option<TransferState>,
    stalled_since: Option<std::time::Instant>,
}

impl TransferStateMachine {
    pub fn new() -> Self {
        Self {
            current: TransferState::Idle,
            previous: None,
            stalled_since: None,
        }
    }
    
    /// Get current state
    pub fn current(&self) -> TransferState {
        self.current
    }
    
    /// Attempt state transition
    pub fn transition(&mut self, next: TransferState) -> Result<TransferState, ProtocolError> {
        if !self.current.can_transition_to(next) {
            return Err(ProtocolError::InvalidStateTransition {
                from: self.current,
                to: next,
            });
        }
        
        self.previous = Some(self.current);
        self.current = next;
        
        if next == TransferState::Stalled {
            self.stalled_since = Some(std::time::Instant::now());
        } else {
            self.stalled_since = None;
        }
        
        Ok(self.current)
    }
    
    /// Force transition (for recovery)
    pub fn force_transition(&mut self, next: TransferState) -> TransferState {
        self.previous = Some(self.current);
        self.current = next;
        self.current
    }
    
    /// Check if transfer is stalled
    pub fn is_stalled(&self) -> bool {
        self.current == TransferState::Stalled
    }
    
    /// Time since stalled
    pub fn stalled_duration(&self) -> Option<std::time::Duration> {
        self.stalled_since.map(|t| t.elapsed())
    }
    
    /// Reset to idle
    pub fn reset(&mut self) {
        self.current = TransferState::Idle;
        self.previous = None;
        self.stalled_since = None;
    }
}

impl Default for TransferStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_transitions() {
        assert!(TransferState::Idle.can_transition_to(TransferState::Scanning));
        assert!(TransferState::Scanning.can_transition_to(TransferState::KeyExchange));
        assert!(TransferState::KeyExchange.can_transition_to(TransferState::Receiving));
        assert!(TransferState::Receiving.can_transition_to(TransferState::Reconstructing));
        assert!(TransferState::Reconstructing.can_transition_to(TransferState::Decrypting));
        assert!(TransferState::Decrypting.can_transition_to(TransferState::Complete));
    }
    
    #[test]
    fn test_invalid_transitions() {
        assert!(!TransferState::Idle.can_transition_to(TransferState::Complete));
        assert!(!TransferState::Complete.can_transition_to(TransferState::Receiving));
    }
    
    #[test]
    fn test_state_machine() {
        let mut sm = TransferStateMachine::new();
        assert_eq!(sm.current(), TransferState::Idle);
        
        sm.transition(TransferState::Scanning).unwrap();
        assert_eq!(sm.current(), TransferState::Scanning);
        
        sm.transition(TransferState::KeyExchange).unwrap();
        assert_eq!(sm.current(), TransferState::KeyExchange);
        
        // Invalid transition should fail
        let result = sm.transition(TransferState::Idle);
        assert!(result.is_err());
    }
}