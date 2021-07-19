use std::io::Write;

use anyhow::{bail, Context, Result};
use cli_table::{print_stdout, Table};
use k256::ecdsa::VerifyingKey;
use solo_machine_core::{
    cosmos::crypto::{PublicKey, PublicKeyAlgo},
    ibc::core::ics24_host::identifier::{ChainId, Identifier},
    service::IbcService,
    DbPool, Event, Signer,
};
use structopt::StructOpt;
use termcolor::{ColorChoice, ColorSpec, StandardStream};
use tokio::sync::mpsc::unbounded_channel;

use crate::command::{add_row, print_stream};

const PUBLIC_KEY_ALGO_VARIANTS: [&str; 2] = ["secp256k1", "eth-secp256k1"];

#[derive(Debug, StructOpt)]
pub enum IbcCommand {
    /// Establishes connection with an IBC enabled chain
    Connect {
        /// Chain ID of IBC enabled chain
        chain_id: ChainId,
        /// Optional memo to include in transactions
        #[structopt(
            long,
            default_value = "solo-machine-memo",
            env = "SOLO_MEMO",
            hide_env_values = true
        )]
        memo: String,
    },
    /// Sends some tokens to IBC enabled chain
    Send {
        /// Chain ID of IBC enabled chain
        chain_id: ChainId,
        /// Amount to send to IBC enabled chain
        amount: u32,
        /// Denom of tokens to send to IBC enabled chain
        denom: Identifier,
        /// Optional receiver address (if this is not provided, tokens will be sent to signer's address)
        receiver: Option<String>,
        /// Optional memo to include in transactions
        #[structopt(
            long,
            default_value = "solo-machine-memo",
            env = "SOLO_MEMO",
            hide_env_values = true
        )]
        memo: String,
    },
    /// Receives some tokens from IBC enabled chain
    Receive {
        /// Chain ID of IBC enabled chain
        chain_id: ChainId,
        /// Amount to receive from IBC enabled chain
        amount: u32,
        /// Denom of tokens to receive from IBC enabled chain
        denom: Identifier,
        /// Optional receiver address (if this is not provided, tokens will be received to signer's address)
        receiver: Option<String>,
        /// Optional memo to include in transactions
        #[structopt(
            long,
            default_value = "solo-machine-memo",
            env = "SOLO_MEMO",
            hide_env_values = true
        )]
        memo: String,
    },
    /// Updates signer's public key on IBC enabled chain for future messages from solo machine
    UpdateSigner {
        /// Chain ID of IBC enabled chain
        chain_id: ChainId,
        /// Hex encoded public key
        #[structopt(long, env = "SOLO_NEW_PUBLIC_KEY", hide_env_values = true)]
        new_public_key: String,
        /// Type of public key
        #[structopt(long, possible_values = &PUBLIC_KEY_ALGO_VARIANTS, default_value = "secp256k1", env = "SOLO_PUBLIC_KEY_ALGO", hide_env_values = true)]
        public_key_algo: PublicKeyAlgo,
        /// Optional memo to include in transactions
        #[structopt(
            long,
            default_value = "solo-machine-memo",
            env = "SOLO_MEMO",
            hide_env_values = true
        )]
        memo: String,
    },
}

impl IbcCommand {
    pub async fn execute(
        self,
        db_pool: DbPool,
        signer: impl Signer,
        color_choice: ColorChoice,
    ) -> Result<()> {
        let (sender, mut receiver) = unbounded_channel();

        let handle = tokio::spawn(async move {
            let mut stdout = StandardStream::stdout(color_choice);

            while let Some(event) = receiver.recv().await {
                match event {
                    Event::TokensSent {
                        chain_id,
                        from_address,
                        to_address,
                        amount,
                        denom,
                    } => {
                        print_stream(&mut stdout, ColorSpec::new().set_bold(true), "Tokens sent!")?;
                        writeln!(stdout)?;

                        let mut table = Vec::new();

                        add_row(&mut table, "Chain ID", chain_id);
                        add_row(&mut table, "From", from_address);
                        add_row(&mut table, "To", to_address);
                        add_row(&mut table, "Amount", amount);
                        add_row(&mut table, "Denom", denom);

                        print_stdout(table.table().color_choice(color_choice))
                            .context("unable to print table to stdout")?;
                    }
                    Event::TokensReceived {
                        chain_id,
                        from_address,
                        to_address,
                        amount,
                        denom,
                    } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            "Tokens received!",
                        )?;
                        writeln!(stdout)?;

                        let mut table = Vec::new();

                        add_row(&mut table, "Chain ID", chain_id);
                        add_row(&mut table, "From", from_address);
                        add_row(&mut table, "To", to_address);
                        add_row(&mut table, "Amount", amount);
                        add_row(&mut table, "Denom", denom);

                        print_stdout(table.table().color_choice(color_choice))
                            .context("unable to print table to stdout")?;
                    }
                    Event::SignerUpdated {
                        chain_id,
                        old_public_key: _,
                        new_public_key: _,
                    } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            "Signer updated!",
                        )?;
                        writeln!(stdout)?;

                        let mut table = Vec::new();

                        add_row(&mut table, "Chain ID", chain_id);

                        print_stdout(table.table().color_choice(color_choice))
                            .context("unable to print table to stdout")?;
                    }
                    Event::CreatedSoloMachineClient { client_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Created solo machine client on IBC enabled chain [Client ID = {}]",
                                client_id
                            ),
                        )?;
                    }
                    Event::CreatedTendermintClient { client_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Created tendermint client on solo machine [Client ID = {}]",
                                client_id
                            ),
                        )?;
                    }
                    Event::InitializedConnectionOnTendermint { connection_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Initialized connection on IBC enabled chain [Connection ID = {}]",
                                connection_id
                            ),
                        )?;
                    }
                    Event::InitializedConnectionOnSoloMachine { connection_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Initialized connection on solo machine [Connection ID = {}]",
                                connection_id
                            ),
                        )?;
                    }
                    Event::ConfirmedConnectionOnTendermint { connection_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Confirmed connection on IBC enabled chain [Connection ID = {}]",
                                connection_id
                            ),
                        )?;
                    }
                    Event::ConfirmedConnectionOnSoloMachine { connection_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Confirmed connection on solo machine [Connection ID = {}]",
                                connection_id
                            ),
                        )?;
                    }
                    Event::InitializedChannelOnTendermint { channel_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Initialized channel on IBC enabled chain [Channel ID = {}]",
                                channel_id
                            ),
                        )?;
                    }
                    Event::InitializedChannelOnSoloMachine { channel_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Initialized channel on solo machine [Channel ID = {}]",
                                channel_id
                            ),
                        )?;
                    }
                    Event::ConfirmedChannelOnTendermint { channel_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Confirmed channel on IBC enabled chain [Channel ID = {}]",
                                channel_id
                            ),
                        )?;
                    }
                    Event::ConfirmedChannelOnSoloMachine { channel_id } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            format!(
                                "Confirmed channel on solo machine [Channel ID = {}]",
                                channel_id
                            ),
                        )?;
                    }
                    Event::ConnectionEstablished {
                        chain_id,
                        connection_details,
                    } => {
                        print_stream(
                            &mut stdout,
                            ColorSpec::new().set_bold(true),
                            "Connection established!",
                        )?;
                        writeln!(stdout)?;

                        let mut table = Vec::new();

                        add_row(&mut table, "Chain ID", chain_id);
                        add_row(
                            &mut table,
                            "Solo machine client ID",
                            connection_details.solo_machine_client_id,
                        );
                        add_row(
                            &mut table,
                            "Tendermint client ID",
                            connection_details.tendermint_client_id,
                        );
                        add_row(
                            &mut table,
                            "Solo machine connection ID",
                            connection_details.solo_machine_connection_id,
                        );
                        add_row(
                            &mut table,
                            "Tendermint connection ID",
                            connection_details.tendermint_connection_id,
                        );
                        add_row(
                            &mut table,
                            "Solo machine channel ID",
                            connection_details.solo_machine_channel_id,
                        );
                        add_row(
                            &mut table,
                            "Tendermint channel ID",
                            connection_details.tendermint_channel_id,
                        );

                        print_stdout(table.table().color_choice(color_choice))
                            .context("unable to print table to stdout")?;
                    }
                    _ => bail!("non-ibc event in ibc command"),
                }
            }

            Ok(())
        });

        {
            let ibc_service = IbcService::new_with_notifier(db_pool, sender);

            match self {
                Self::Connect { chain_id, memo } => {
                    ibc_service.connect(signer, chain_id, memo).await
                }
                Self::Send {
                    chain_id,
                    amount,
                    denom,
                    receiver,
                    memo,
                } => {
                    ibc_service
                        .send_to_chain(signer, chain_id, amount, denom, receiver, memo)
                        .await
                }
                Self::Receive {
                    chain_id,
                    amount,
                    denom,
                    receiver,
                    memo,
                } => {
                    ibc_service
                        .receive_from_chain(signer, chain_id, amount, denom, receiver, memo)
                        .await
                }
                Self::UpdateSigner {
                    chain_id,
                    new_public_key,
                    public_key_algo,
                    memo,
                } => {
                    let new_public_key_bytes =
                        hex::decode(&new_public_key).context("unable to decode hex bytes")?;

                    let new_verifying_key = VerifyingKey::from_sec1_bytes(&new_public_key_bytes)
                        .context("invalid secp256k1 bytes")?;

                    let new_public_key = match public_key_algo {
                        PublicKeyAlgo::Secp256k1 => PublicKey::Secp256k1(new_verifying_key),
                        #[cfg(feature = "ethermint")]
                        PublicKeyAlgo::EthSecp256k1 => PublicKey::EthSecp256k1(new_verifying_key),
                    };

                    ibc_service
                        .update_signer(signer, chain_id, new_public_key, memo)
                        .await
                }
            }?;
        }

        handle.await.context("unable to join async task")?
    }
}