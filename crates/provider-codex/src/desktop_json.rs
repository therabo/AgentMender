//! Bounded JSON syntax tree retaining original byte ranges for fixed reducers.
use std::collections::BTreeSet;

#[derive(Debug)]
pub(super) struct Node {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}
#[derive(Debug)]
pub(super) enum Kind {
    Object(Vec<(String, usize, Node)>),
    Array(Vec<Node>),
    Scalar,
}

pub(super) fn parse(bytes: &[u8]) -> Result<Node, ()> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(());
    }
    // serde validates UTF-8, numeric grammar and escape sequences. The parser
    // below records ranges only and enforces stricter duplicate-key handling.
    let _: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    let mut p = Parser {
        bytes,
        pos: 0,
        nodes: 0,
    };
    let node = p.node(0)?;
    p.ws();
    if p.pos != bytes.len() {
        return Err(());
    }
    Ok(node)
}
struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_whitespace())
        {
            self.pos += 1;
        }
    }
    fn string(&mut self) -> Result<String, ()> {
        let start = self.pos;
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(());
        }
        self.pos += 1;
        while let Some(&c) = self.bytes.get(self.pos) {
            self.pos += 1;
            if c == b'\\' {
                self.pos += 1;
            } else if c == b'"' {
                return serde_json::from_slice(&self.bytes[start..self.pos]).map_err(|_| ());
            }
        }
        Err(())
    }
    fn node(&mut self, depth: usize) -> Result<Node, ()> {
        if depth > 128 || self.nodes >= 1_000_000 {
            return Err(());
        }
        self.nodes += 1;
        self.ws();
        let start = self.pos;
        let kind = match self.bytes.get(self.pos).copied().ok_or(())? {
            b'{' => {
                self.pos += 1;
                self.ws();
                let mut fields = Vec::new();
                let mut keys = BTreeSet::new();
                while self.bytes.get(self.pos) != Some(&b'}') {
                    self.ws();
                    let pair = self.pos;
                    let key = self.string()?;
                    if !keys.insert(key.clone()) {
                        return Err(());
                    }
                    self.ws();
                    if self.bytes.get(self.pos) != Some(&b':') {
                        return Err(());
                    }
                    self.pos += 1;
                    let value = self.node(depth + 1)?;
                    fields.push((key, pair, value));
                    self.ws();
                    if self.bytes.get(self.pos) == Some(&b',') {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                if self.bytes.get(self.pos) != Some(&b'}') {
                    return Err(());
                }
                self.pos += 1;
                Kind::Object(fields)
            }
            b'[' => {
                self.pos += 1;
                self.ws();
                let mut items = Vec::new();
                while self.bytes.get(self.pos) != Some(&b']') {
                    items.push(self.node(depth + 1)?);
                    self.ws();
                    if self.bytes.get(self.pos) == Some(&b',') {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                if self.bytes.get(self.pos) != Some(&b']') {
                    return Err(());
                }
                self.pos += 1;
                Kind::Array(items)
            }
            b'"' => {
                self.string()?;
                Kind::Scalar
            }
            _ => {
                while self.bytes.get(self.pos).is_some_and(|b| {
                    !matches!(b, b',' | b']' | b'}' | b' ' | b'\t' | b'\r' | b'\n')
                }) {
                    self.pos += 1;
                }
                Kind::Scalar
            }
        };
        Ok(Node {
            start,
            end: self.pos,
            kind,
        })
    }
}
impl Node {
    pub fn field(&self, key: &str) -> Result<Option<&Node>, ()> {
        let Kind::Object(fields) = &self.kind else {
            return Err(());
        };
        Ok(fields.iter().find(|(k, _, _)| k == key).map(|(_, _, n)| n))
    }
    pub fn string(&self, bytes: &[u8]) -> Option<String> {
        serde_json::from_slice(&bytes[self.start..self.end]).ok()
    }
    pub fn object(&self) -> Result<&[(String, usize, Node)], ()> {
        match &self.kind {
            Kind::Object(v) => Ok(v),
            _ => Err(()),
        }
    }
    pub fn array(&self) -> Result<&[Node], ()> {
        match &self.kind {
            Kind::Array(v) => Ok(v),
            _ => Err(()),
        }
    }
}
/// Preserve every retained value's exact byte span. Separator whitespace around
/// a removed run is necessarily removed along with its comma.
pub(super) fn remove_runs(
    spans: &[(usize, usize)],
    selected: &[bool],
    edits: &mut Vec<(usize, usize)>,
) {
    let mut i = 0;
    while i < selected.len() {
        if !selected[i] {
            i += 1;
            continue;
        }
        let first = i;
        while i + 1 < selected.len() && selected[i + 1] {
            i += 1;
        }
        let last = i;
        if first == 0 && last + 1 < spans.len() {
            edits.push((spans[first].0, spans[last + 1].0));
        } else if first > 0 {
            edits.push((spans[first - 1].1, spans[last].1));
        } else {
            edits.push((spans[first].0, spans[last].1));
        }
        i += 1;
    }
}
