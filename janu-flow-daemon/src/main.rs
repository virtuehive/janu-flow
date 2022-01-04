// Copyright (c) 2017, 2021 Tawedge.
//
// This program and the accompanying materials are made available under the
// terms of the TAW Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   open
//
use structopt::StructOpt;

use async_ctrlc::CtrlC;
use std::str;

use async_std::fs;
use async_std::path::Path;
use async_std::prelude::*;

mod daemon;
use daemon::Daemon;
use janu_flow::runtime::RuntimeConfig;

static RUNTIME_CONFIG_FILE: &str = "/etc/janu-flow/runtime.yaml";

const GIT_VERSION: &str = git_version::git_version!(prefix = "v", cargo_prefix = "v");

#[derive(Debug, StructOpt)]
#[structopt(name = "dpn")]
struct RuntimeOpt {
    #[structopt(short = "c", long = "configuration", default_value = RUNTIME_CONFIG_FILE)]
    config: String,
    #[structopt(short = "v", long = "version")]
    print_version: bool,
    #[structopt(short = "i", long = "node_uuid")]
    node_uuid: bool,
}

async fn read_file(path: &Path) -> String {
    fs::read_to_string(path).await.unwrap()
}

async fn _write_file(path: &Path, content: Vec<u8>) {
    let mut file = fs::File::create(path).await.unwrap();
    file.write_all(&content).await.unwrap();
    file.sync_all().await.unwrap();
}

#[async_std::main]
async fn main() {
    // Init logging
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let args = RuntimeOpt::from_args();

    if args.print_version {
        println!("Janu Flow runtime version: {}", GIT_VERSION);
        std::process::exit(0);
    }

    if args.node_uuid {
        println!("{}", daemon::get_machine_uuid().unwrap());
        std::process::exit(0);
    }

    let conf_file_path = Path::new(&args.config);
    let config = serde_yaml::from_str::<RuntimeConfig>(&(read_file(conf_file_path).await)).unwrap();

    let rt = Daemon::from_config(config).unwrap();

    let (s, h) = rt.start().await.unwrap();

    let ctrlc = CtrlC::new().expect("Unable to create Ctrl-C handler");
    let mut stream = ctrlc.enumerate().take(1);
    stream.next().await;
    log::trace!("Received Ctrl-C start teardown");

    //Here we send the stop signal to the rt object and waits that it ends
    rt.stop(s).await.unwrap();

    //wait for the futures to ends
    h.await.unwrap();
}
