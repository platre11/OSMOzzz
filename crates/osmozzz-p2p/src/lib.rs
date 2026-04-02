pub mod discovery;
pub mod history;
pub mod identity;
pub mod node;
pub mod permissions;
pub mod protocol;
pub mod store;

pub use identity::PeerIdentity;
pub use node::P2pNode;
pub use permissions::{PeerPermissions, SharedSource, ToolAccessMode};
pub use protocol::{PeerInfo, SearchRequest, SearchResponse, ToolCallRequest, ToolCallResult};
pub use store::{KnownPeer, PeerStore};
pub use history::{QueryHistoryEntry, QueryHistoryLog};
