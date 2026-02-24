use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use glam::Vec3;
use anyhow::Result;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Atom {
    pub serial: i32,
    pub name: String,
    pub res_name: String,
    pub chain_id: char,
    pub res_seq: i32,
    pub pos: Vec3,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Model {
    pub atoms: Vec<Atom>,
    pub connections: Vec<bool>, // true if atom[i] connects to atom[i+1]
    pub center: Vec3,
}

impl Model {
    pub fn new() -> Self {
        Model {
            atoms: Vec::new(),
            connections: Vec::new(),
            center: Vec3::ZERO,
        }
    }
}

pub fn read_pdb<P: AsRef<Path>>(path: P) -> Result<Vec<Model>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let protein_bb = ["N", "CA", "C"];
    let nucleic_bb = ["P", "O5'", "C5'", "C4'", "C3'", "O3'"];

    // We only care about models if explicit MODEL record exists, otherwise it's one model.
    // For simplicity, we'll just read all atoms into one list per MODEL block.
    // If no MODEL tags, it's one model.

    let mut models = Vec::new();
    let mut current_atoms = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("MODEL") {
            if !current_atoms.is_empty() {
                models.push(process_model(current_atoms));
                current_atoms = Vec::new();
            }
        } else if line.starts_with("ENDMDL") {
            if !current_atoms.is_empty() {
                models.push(process_model(current_atoms));
                current_atoms = Vec::new();
            }
        } else if line.starts_with("ATOM") || line.starts_with("HETATM") {
            if let Some(atom) = parse_atom_line(&line) {
                // Filter
                let name = atom.name.trim();
                if protein_bb.contains(&name) || nucleic_bb.contains(&name) {
                    current_atoms.push(atom);
                }
            }
        }
    }

    // Handle case where no MODEL/ENDMDL tags or last model
    if !current_atoms.is_empty() {
        models.push(process_model(current_atoms));
    }

    if models.is_empty() {
        anyhow::bail!("No atoms found or parsed.");
    }

    Ok(models)
}

fn parse_atom_line(line: &str) -> Option<Atom> {
    if line.len() < 54 { return None; }

    // Columns (0-indexed):
    // 6-11: Serial (integer) -> line[6..11]
    // 12-16: Name (string) -> line[12..16]
    // 17-20: ResName (string) -> line[17..20]
    // 21: ChainID (char) -> line[21..22]
    // 22-26: ResSeq (integer) -> line[22..26]
    // 30-38: X (float) -> line[30..38]
    // 38-46: Y (float) -> line[38..46]
    // 46-54: Z (float) -> line[46..54]

    let serial_str = line.get(6..11)?.trim();
    let name_str = line.get(12..16)?.trim();
    let res_name_str = line.get(17..20)?.trim();
    let chain_id_str = line.get(21..22)?;
    let res_seq_str = line.get(22..26)?.trim();
    let x_str = line.get(30..38)?.trim();
    let y_str = line.get(38..46)?.trim();
    let z_str = line.get(46..54)?.trim();

    let serial = serial_str.parse().ok()?;
    let res_seq = res_seq_str.parse().ok()?;
    let x: f32 = x_str.parse().ok()?;
    let y: f32 = y_str.parse().ok()?;
    let z: f32 = z_str.parse().ok()?;

    let chain_id = chain_id_str.chars().next().unwrap_or(' ');

    Some(Atom {
        serial,
        name: name_str.to_string(),
        res_name: res_name_str.to_string(),
        chain_id,
        res_seq,
        pos: Vec3::new(x, y, z),
    })
}

fn process_model(mut atoms: Vec<Atom>) -> Model {
    if atoms.is_empty() {
        return Model::new();
    }

    // Calculate center
    let mut sum = Vec3::ZERO;
    for atom in &atoms {
        sum += atom.pos;
    }
    let center = sum / atoms.len() as f32;

    // Center atoms
    for atom in &mut atoms {
        atom.pos -= center;
    }

    // Calculate connections
    let mut connections = Vec::new();
    for i in 0..atoms.len() {
        if i == atoms.len() - 1 {
            // Last atom has no next atom to connect to, usually connections has length N-1 or N
            // The drawing logic iterates 0..N-1. So connections should be size N-1?
            // Python: "connections.append(...)" for each atom if it connects to previous?
            // Python logic:
            // for atom in model:
            //   if len(model_coords) > 0:
            //      connections.append(chain == last_chain and (res == last_res + 1 or res == last_res))
            //   model_coords.append(atom)

            // So connection[i] corresponds to connection between atom[i] and atom[i+1]?
            // Python: connections list length is N-1 (it appends when len > 0).
            // Actually, in python:
            //   if len(model_coords) > 0: append connection.
            //   model_coords.append.
            // So if N atoms, N-1 connections.
            // connection[0] is between atom[0] and atom[1]?
            // No, wait.
            // atom 0: len=0, no append. list becomes [a0].
            // atom 1: len=1 > 0. append conn (based on a1 and a0). list [a0, a1].
            // So connection[0] is the link between a1 and a0 (the previous one).

            // Wait, let's look at Python view loop:
            // for i in range(coords.shape[1] - 1):
            //    if not info['connections'][i]: continue
            //    start = coords[i], end = coords[i+1]

            // So connections[i] describes link between i and i+1.
            // But in Python parse loop:
            // when processing atom `i` (which is the i-th appended, so index i in array),
            // we check against `last_chain`, `last_res`.
            // `connections.append(...)` happens before appending the current atom.
            // So at index i=1 (2nd atom), we append `connections[0]`.
            // This checks `current` vs `last`. i.e. atom[1] vs atom[0].
            // So connections[0] describes link between 0 and 1.
            // Yes.
            break;
        }

        let curr = &atoms[i];
        let next = &atoms[i+1];

        let connected = if curr.chain_id == next.chain_id {
            if curr.res_seq == next.res_seq {
                true // Same residue
            } else if next.res_seq == curr.res_seq + 1 {
                true // Sequential residue
            } else {
                false
            }
        } else {
            false
        };
        connections.push(connected);
    }

    Model {
        atoms,
        connections,
        center,
    }
}
