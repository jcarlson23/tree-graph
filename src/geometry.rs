use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize,Deserialize,Debug,Copy,Clone,PartialEq)]
pub struct GPoint 
{
   pub row: usize, 
   pub column: usize,
}

#[derive(Serialize,Deserialize,Debug,Copy,Clone,PartialEq)]
pub struct GRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_point: GPoint,
    pub end_point: GPoint,
}

impl From<tree_sitter::Point> for GPoint {
   fn from(point:tree_sitter::Point) -> GPoint {
       GPoint {
           row: point.row,
           column: point.column,
       }
   }
}

impl From<tree_sitter::Range> for GRange {
   fn from(value:tree_sitter::Range) -> GRange {
       GRange {
           start_byte: value.start_byte,
           end_byte: value.end_byte,
           start_point: GPoint::from(value.start_point),
           end_point: GPoint::from(value.end_point),
       }
   }
}

#[derive(Serialize,Deserialize,Debug,Copy,Clone)]
pub struct GNode { // G-Node to differentiate from a Treesitter node
    pub id: usize,
    pub kind_id: u16,
    pub range: GRange,
}

// #[cfg(feature="informational")]
impl fmt::Display for GNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GNode({}, {})", self.kind_id, self.range.start_byte)
    }
}


#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Edge {
    #[serde(serialize_with = "serialize_node_index", deserialize_with = "deserialize_node_index")]
    pub source: NodeIndex,
    #[serde(serialize_with = "serialize_node_index", deserialize_with = "deserialize_node_index")]
    pub target: NodeIndex,
}

fn serialize_node_index<S>(node_index: &NodeIndex, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer
{
    let index = node_index.index();
    index.serialize(s)
}

fn deserialize_node_index<'de, D>(d: D) -> Result<NodeIndex, D::Error>
where
    D: serde::Deserializer<'de>
{
    let index = usize::deserialize(d)?;
    Ok(NodeIndex::new(index))
}
