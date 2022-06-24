use crate::props::skip_proof;
use hdk::prelude::holo_hash::AgentPubKeyB64;
use hdk::prelude::*;

/// check if the instance that is making the call is eligible
pub fn is_read_only_instance() -> bool {
    if skip_proof() {
        return false;
    }
    if let Ok(entries) = &query(ChainQueryFilter::new().action_type(ActionType::AgentValidationPkg))
    {
        if let Action::AgentValidationPkg(h) = entries[0].action() {
            if let Some(mem_proof) = &h.membrane_proof {
                if is_read_only_proof(&mem_proof) {
                    return true;
                }
            }
        }
    };
    false
}

/// check to see if this is the valid read_only membrane proof
pub fn is_read_only_proof(mem_proof: &MembraneProof) -> bool {
    let b = mem_proof.bytes();
    b == &[0]
}

/// This is the current structure of the payload the holo signs
#[hdk_entry_helper]
#[derive(Clone)]
pub struct JoiningCodePayload {
    pub role: String,
    pub record_locator: String,
    pub registered_agent: AgentPubKeyB64,
}

#[hdk_entry_defs]
#[unit_enum(EntryTypesUnit)]
pub enum EntryTypes {
    JoiningCodePayload(JoiningCodePayload),
}

/// Validate joining code from the membrane_proof
pub fn validate_joining_code(
    progenitor_agent: AgentPubKey,
    author: AgentPubKey,
    membrane_proof: Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    match membrane_proof {
        Some(mem_proof) => {
            if is_read_only_proof(&mem_proof) {
                return Ok(ValidateCallbackResult::Valid);
            };
            // TODO: find a way to TryFrom a ref, to avoid cloning.
            let mem_proof = match Record::try_from((*mem_proof).clone()) {
                Ok(r) => r,
                Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(e.to_string()))),
            };

            trace!("Joining code provided: {:?}", mem_proof);

            let joining_code_author = mem_proof.action().author().clone();

            if joining_code_author != progenitor_agent {
                trace!("Joining code validation failed");
                return Ok(ValidateCallbackResult::Invalid(format!(
                    "Joining code invalid: unexpected author ({:?})",
                    joining_code_author
                )));
            }

            let e = mem_proof.entry();
            if let RecordEntry::Present(entry) = e {
                let signature = mem_proof.signature().clone();
                match verify_signature(progenitor_agent, signature, mem_proof.action()) {
                    Ok(verified) => {
                        if verified {
                            // check that the joining code has the correct author key in it
                            // once this is added to the registration flow, e.g.:
                            let joining_code = JoiningCodePayload::try_from(entry)?;
                            if AgentPubKey::from(joining_code.registered_agent) != author {
                                return Ok(ValidateCallbackResult::Invalid(
                                    "Joining code invalid: incorrect registered agent key"
                                        .to_string(),
                                ));
                            }
                            trace!("Joining code validated");
                            Ok(ValidateCallbackResult::Valid)
                        } else {
                            trace!("Joining code validation failed: incorrect signature");
                            Ok(ValidateCallbackResult::Invalid(
                                "Joining code invalid: incorrect signature".to_string(),
                            ))
                        }
                    }
                    Err(e) => {
                        debug!("Error on get when verifying signature of agent entry: {:?}; treating as unresolved dependency",e);
                        Ok(ValidateCallbackResult::UnresolvedDependencies(vec![
                            (author).into(),
                        ]))
                    }
                }
            } else {
                Ok(ValidateCallbackResult::Invalid(
                    "Joining code invalid payload".to_string(),
                ))
            }
        }
        None => Ok(ValidateCallbackResult::Invalid(
            "No membrane proof found".to_string(),
        )),
    }
}
