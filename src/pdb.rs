use std::path::Path;
use glam::Vec3;
use anyhow::Result;
use pdbtbx::{self, ReadOptions, Format};

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

pub fn read_pdb<P: AsRef<Path>>(path: P, chain_filter: Option<&str>, format_str: Option<&str>) -> Result<Vec<Model>> {
    let path_str = path.as_ref().to_str().ok_or_else(|| anyhow::anyhow!("Invalid path string"))?;

    // Determine format and read
    let (pdb, _errors) = if let Some(fmt) = format_str {
        let format_enum = match fmt.to_lowercase().as_str() {
            "pdb" => Format::Pdb,
            "mmcif" | "cif" => Format::Mmcif,
            _ => anyhow::bail!("Unsupported format: {}", fmt),
        };
        ReadOptions::default()
            .set_format(format_enum)
            .read(path_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse file: {:?}", e))?
    } else {
        pdbtbx::open(path_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse PDB file: {:?}", e))?
    };

    let protein_bb = ["N", "CA", "C"];
    let nucleic_bb = ["P", "O5'", "C5'", "C4'", "C3'", "O3'"];

    let mut models = Vec::new();

    for pdb_model in pdb.models() {
        let mut atoms = Vec::new();

        for chain in pdb_model.chains() {
            let cid_str = chain.id();

            if let Some(c_filter) = chain_filter {
                if cid_str != c_filter {
                    continue;
                }
            }

            let chain_id = cid_str.chars().next().unwrap_or(' ');

            for residue in chain.residues() {
                // residue.name() returns Option<&str> or &str?
                // residue.serial_number() returns isize usually.
                let res_name = residue.name().unwrap_or("UNK").to_string();
                let res_seq = residue.serial_number() as i32;

                for atom in residue.atoms() {
                    let name = atom.name();
                    let name_str = name.trim();

                    if protein_bb.contains(&name_str) || nucleic_bb.contains(&name_str) {
                        let (x, y, z) = atom.pos();
                        let x = x as f32;
                        let y = y as f32;
                        let z = z as f32;

                        atoms.push(Atom {
                            serial: atom.serial_number() as i32,
                            name: name.to_string(),
                            res_name: res_name.clone(),
                            chain_id,
                            res_seq,
                            pos: Vec3::new(x, y, z),
                        });
                    }
                }
            }
        }

        // Only add model if it has atoms
        if !atoms.is_empty() {
            models.push(process_model(atoms));
        }
    }

    if models.is_empty() {
        anyhow::bail!("No atoms found or parsed.");
    }

    Ok(models)
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
