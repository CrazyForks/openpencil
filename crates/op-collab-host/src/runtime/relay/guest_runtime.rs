use super::*;

/// Coarse, credential-free startup stages for diagnosing a public Relay join.
///
/// Never add request data, endpoint URLs, invite material, identities, keys, or
/// raw transport errors to this enum. Its `Debug` output is emitted to the
/// local terminal when startup fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelayGuestStartupStage {
    Bootstrap,
    PairingClaim,
    RegionSelection,
    Clock,
    InviteVerification,
    LocalAuthentication,
    RelayBridge,
}

struct RelayGuestStageFailure {
    stage: RelayGuestStartupStage,
    failure: CollabRuntimeFailure,
}

impl std::fmt::Display for RelayGuestStageFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "RelayGuestStageFailed {{ stage: {:?}, failure: {:?} }}",
            self.stage, self.failure
        )
    }
}

fn at_stage<T>(
    stage: RelayGuestStartupStage,
    result: Result<T, CollabRuntimeFailure>,
) -> Result<T, CollabRuntimeFailure> {
    at_stage_with(stage, result, |diagnostic| {
        eprintln!("[collab] {diagnostic}");
    })
}

fn at_stage_with<T>(
    stage: RelayGuestStartupStage,
    result: Result<T, CollabRuntimeFailure>,
    report: impl FnOnce(RelayGuestStageFailure),
) -> Result<T, CollabRuntimeFailure> {
    match result {
        Ok(value) => Ok(value),
        Err(failure) => {
            report(RelayGuestStageFailure { stage, failure });
            Err(failure)
        }
    }
}

impl GuestRelayRuntime {
    pub(in crate::runtime) fn start(
        request: &RelayGuestRequest,
        key: Arc<DeviceStaticKey>,
        local: Arc<std::sync::RwLock<LocalAdmission>>,
    ) -> Result<Self, CollabRuntimeFailure> {
        // Resolve and verify the invite on the guest network worker. The UI
        // only parses enough of the bounded invite to select its claimed
        // region and render status.
        let bootstrap = at_stage(RelayGuestStartupStage::Bootstrap, request.provider.load())?;
        let (invite, home_region) = match &request.secret {
            RelayJoinSecret::Invite(invite) => (invite.as_ref().clone(), request.home_region),
            RelayJoinSecret::Pairing(code) => {
                let invite = at_stage(
                    RelayGuestStartupStage::PairingClaim,
                    claim_pairing_invite(
                        &bootstrap,
                        code,
                        request.control_plane.as_ref(),
                        &key,
                        &local,
                    ),
                )?;
                let claimed_region = invite.locator().claims().home_region();
                (invite, claimed_region)
            }
        };
        let region = at_stage(
            RelayGuestStartupStage::RegionSelection,
            bootstrap.region(home_region),
        )?;
        let endpoint = region.relay_endpoint.clone();
        let development_unsigned = development_unsigned_allowed(&endpoint);
        let now = at_stage(
            RelayGuestStartupStage::Clock,
            unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable),
        )? / 1_000;
        let route = at_stage(RelayGuestStartupStage::InviteVerification, {
            if development_unsigned {
                invite
                    .verify(&AcceptAllDevelopmentLocator, now)
                    .map_err(invite_verify_failure)
            } else {
                invite
                    .verify(&region.locator_verifier, now)
                    .map_err(invite_verify_failure)
            }
        })?;
        // No region cross-check here: `home_region` is itself derived from the
        // (canonically parsed) invite claims on both branches, so the verify
        // step above is the authoritative gate.
        let expected_discovery_id = route
            .locator()
            .claims()
            .expected_discovery_id()
            .as_str()
            .to_owned();
        let expected_remote_static = *route.locator().claims().owner_noise_static().as_bytes();
        let auth = at_stage(
            RelayGuestStartupStage::LocalAuthentication,
            LocalAdmission::relay_auth_extension(*key.public_key()).map_err(|error| error.failure),
        )?;
        let handshake = RelayHandshake::new(route, auth);
        let authenticator = if development_unsigned {
            None
        } else {
            Some(at_stage(
                RelayGuestStartupStage::LocalAuthentication,
                LocalAdmission::challenge_bound_relay_authenticator(
                    local,
                    key,
                    Arc::clone(&region.relay_x25519_keys),
                )
                .map_err(|error| error.failure),
            )?)
        };
        let bridge = at_stage(
            RelayGuestStartupStage::RelayBridge,
            crate::blocking::block_on(async move {
                if development_unsigned {
                    start_development_guest_bridge(endpoint, handshake).await
                } else {
                    RelayGuestBridge::start(
                        endpoint,
                        handshake,
                        authenticator.ok_or(CollabRuntimeFailure::RelayUnavailable)?,
                    )
                    .await
                    .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
                }
            }),
        )?;
        let local_addr = bridge.local_addr();
        Ok(Self {
            local_addr,
            expected_discovery_id,
            expected_remote_static,
            bridge,
        })
    }

    pub(in crate::runtime) const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub(in crate::runtime) fn bridge_diagnostic(
        &self,
    ) -> (
        op_collab_relay_client::RelayBridgePhase,
        Option<op_collab_relay_client::RelayFailureKind>,
    ) {
        let status = self.bridge.status();
        (status.phase, status.last_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_wrapper_emits_only_the_safe_failure_shape() {
        let mut emitted = None;
        let result = at_stage_with::<()>(
            RelayGuestStartupStage::PairingClaim,
            Err(CollabRuntimeFailure::RelayUnavailable),
            |diagnostic| emitted = Some(diagnostic.to_string()),
        );

        assert_eq!(result, Err(CollabRuntimeFailure::RelayUnavailable));
        assert_eq!(
            emitted.as_deref(),
            Some("RelayGuestStageFailed { stage: PairingClaim, failure: RelayUnavailable }")
        );
    }

    #[test]
    fn successful_stage_emits_nothing() {
        let mut emitted = false;
        let result = at_stage_with(RelayGuestStartupStage::Bootstrap, Ok(7_u8), |_| {
            emitted = true
        });

        assert_eq!(result, Ok(7));
        assert!(!emitted);
    }
}
