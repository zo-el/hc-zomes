mod entries;
mod handler;
use entries::*;
use hdk::prelude::*;
mod error;
mod validation;
use hc_utils::WrappedAgentPubKey;

#[hdk_extern]
fn update_my_profile(profile_input: ProfileInput) -> ExternResult<Profile> {
    Ok(handler::__update_my_profile(profile_input)?)
}

#[hdk_extern]
fn get_my_profile(_: ()) -> ExternResult<Profile> {
    Ok(handler::__get_my_profile()?)
}

#[hdk_extern]
fn get_profile(agent_address: WrappedAgentPubKey) -> ExternResult<Profile> {
    Ok(handler::__get_profile(AgentPubKey::from(agent_address))?)
}

#[hdk_extern]
fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op {
        Op::StoreEntry {
            entry: Entry::Agent(_),
            ..
        } => Ok(ValidateCallbackResult::Valid),
        Op::StoreEntry {
            entry,
            action:
                SignedHashed {
                    hashed:
                        HoloHashed {
                            content: action, ..
                        },
                    ..
                },
        } => validation::__validate_entry(entry, action.author()),
        Op::RegisterDelete { .. } => Ok(ValidateCallbackResult::Invalid(
            "Invalid try to delete an Entry".to_string(),
        )),
        Op::RegisterUpdate {
            new_entry,
            update:
                SignedHashed {
                    hashed:
                        HoloHashed {
                            content: action, ..
                        },
                    ..
                },
            ..
        } => {
            if is_not_editable() {
                Ok(ValidateCallbackResult::Invalid(
                    "Invalid try to Delete Entry".to_string(),
                ))
            } else {
                validation::__validate_entry(new_entry, &action.author)
            }
        }
        Op::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Invalid(
            "Invalid try to update Link".to_string(),
        )),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

#[derive(Debug, Serialize, Deserialize, SerializedBytes, Clone)]
struct Props {
    not_editable_profile: bool,
}

/// Checking properties for `not_editable_profile` flag
pub fn is_not_editable() -> bool {
    if let Ok(info) = dna_info() {
        return is_not_editable_sb(&info.properties);
    }
    false
}

/// Deserialize properties into the Props expected by this zome
pub fn is_not_editable_sb(encoded_props: &SerializedBytes) -> bool {
    let maybe_props = Props::try_from(encoded_props.to_owned());
    if let Ok(props) = maybe_props {
        return props.not_editable_profile;
    }
    false
}
