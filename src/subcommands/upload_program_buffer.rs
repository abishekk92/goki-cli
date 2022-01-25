use anchor_client::Cluster;
use anyhow::format_err;
use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signature::Signer;
use std::fs::File;
use std::io::BufReader;
use tempfile::NamedTempFile;

use crate::location::fetch_program_file;
use crate::solana_cmd;
use crate::utils::exec_command_with_output;
use crate::utils::gen_new_keypair;
use crate::utils::get_cluster_url;
use crate::utils::get_deployer_kp_path;
use crate::utils::print_header;
use crate::utils::sha256_digest;

#[derive(Serialize, Deserialize)]
struct ProgramInfo {
    pub authority: String,
}

pub async fn process(cluster: Cluster, location: String, program_id: String) -> Result<()> {
    let deployer_kp_path = get_deployer_kp_path(&cluster)?;

    let mut program_file = NamedTempFile::new()?;
    fetch_program_file(&mut program_file, location.as_str()).await?;

    let input = File::open(program_file.path())?;
    let mut reader = BufReader::new(input);
    let (program_file_size, program_file_digest) = sha256_digest(&mut reader)?;
    println!("Program buffer downloaded.");
    println!("Size (bytes): {}", program_file_size.to_string().green());
    println!("SHA256: {}", program_file_digest.green());

    let mut buffer_kp_file = NamedTempFile::new()?;
    let buffer_key = gen_new_keypair(&mut buffer_kp_file)?;

    let deployer_kp =
        read_keypair_file(&deployer_kp_path).map_err(|_| format_err!("invalid keypair"))?;
    println!(
        "Uploading program buffer to cluster {} with signer {}",
        cluster,
        deployer_kp.pubkey()
    );
    println!("Make sure to send enough lamports to this address for the deploy.");

    let program_info_output = exec_command_with_output(
        std::process::Command::new("solana")
            .arg("--url")
            .arg(get_cluster_url(&cluster)?)
            .arg("--keypair")
            .arg(&deployer_kp_path)
            .arg("program")
            .arg("show")
            .arg(&program_id)
            .arg("--output")
            .arg("json-compact"),
    )?;
    let program_info: ProgramInfo = serde_json::from_str(program_info_output.as_str())?;

    println!("Program ID: {}", program_id);
    println!("Program authority: {}", program_info.authority);
    println!("Buffer key: {}", buffer_key);

    print_header("Writing buffer");

    solana_cmd::write_buffer(&cluster, program_file.path(), buffer_kp_file.path())?;

    print_header("Setting buffer authority");

    solana_cmd::set_buffer_authority(&cluster, &buffer_key, &program_info.authority)?;

    println!("Buffer upload complete.");
    println!("Buffer: {}", buffer_key.to_string().green());
    println!("SHA256: {}", program_file_digest.green());

    Ok(())
}
