use std::{env, path::PathBuf, process::ExitCode};

use op_collab_relay_locator_hsm::{
    secure_file, server, KeyStore, SignerConfig, SignerError, SignerResult,
};
use tracing_subscriber::EnvFilter;

enum Command {
    Serve,
    Check,
    Public,
    Provision { kid: String },
    Initialize { so_pin_file: PathBuf },
}

struct Arguments {
    command: Command,
    config: PathBuf,
}

fn main() -> ExitCode {
    init_tracing();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("locator HSM signer failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> SignerResult<()> {
    let arguments = parse_arguments()?;
    let config_bytes = secure_file::read_config(&arguments.config)?;
    let config = SignerConfig::from_bytes(&config_bytes)?;
    let pin = secure_file::read_pin(&config.pin_file)?;
    match arguments.command {
        Command::Serve => {
            let store = KeyStore::open(&config, &pin)?;
            server::serve(&config, &store)
        }
        Command::Check => {
            let store = KeyStore::open(&config, &pin)?;
            store.readiness_canary()?;
            server::probe_socket(&config)?;
            println!("ready");
            Ok(())
        }
        Command::Public => {
            let store = KeyStore::open(&config, &pin)?;
            println!("{}", serde_json::to_string_pretty(&store.public_key_set())?);
            Ok(())
        }
        Command::Provision { kid } => {
            let key = config.key(&kid).ok_or_else(|| {
                SignerError::Config("--kid must identify one configured rotation key".into())
            })?;
            let record = KeyStore::provision(&config, &pin, key)?;
            println!("{}", serde_json::to_string_pretty(&record)?);
            Ok(())
        }
        Command::Initialize { so_pin_file } => {
            let so_pin = secure_file::read_pin(&so_pin_file)?;
            KeyStore::initialize_token(&config, &pin, &so_pin)
        }
    }
}

fn parse_arguments() -> SignerResult<Arguments> {
    let mut arguments = env::args().skip(1);
    let command = match arguments.next().as_deref() {
        Some("serve") => Command::Serve,
        Some("check") => Command::Check,
        Some("public") => Command::Public,
        Some("provision") => Command::Provision { kid: String::new() },
        Some("initialize") => Command::Initialize {
            so_pin_file: PathBuf::new(),
        },
        _ => return Err(usage()),
    };
    let mut config = None;
    let mut kid = None;
    let mut so_pin_file = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--config" if config.is_none() => config = arguments.next().map(PathBuf::from),
            "--kid" if kid.is_none() => kid = arguments.next(),
            "--so-pin-file" if so_pin_file.is_none() => {
                so_pin_file = arguments.next().map(PathBuf::from)
            }
            _ => return Err(usage()),
        }
    }
    let config = config.ok_or_else(usage)?;
    let command = match command {
        Command::Provision { .. } if so_pin_file.is_none() => Command::Provision {
            kid: kid.ok_or_else(usage)?,
        },
        Command::Initialize { .. } if kid.is_none() => Command::Initialize {
            so_pin_file: so_pin_file.ok_or_else(usage)?,
        },
        other if kid.is_none() && so_pin_file.is_none() => other,
        _ => return Err(usage()),
    };
    Ok(Arguments { command, config })
}

fn usage() -> SignerError {
    SignerError::Config(
        "usage: op-collab-relay-locator-hsm <serve|check|public> --config PATH; \
         provision --config PATH --kid KID; or initialize --config PATH --so-pin-file PATH"
            .into(),
    )
}

fn init_tracing() {
    let filter = env::var("OPENPENCIL_LOCATOR_HSM_LOG")
        .ok()
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();
}
