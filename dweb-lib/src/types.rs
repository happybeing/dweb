/*
*   Copyright (c) 2025- Mark Hughes

*   This program is free software: you can redistribute it and/or modify
*   it under the terms of the GNU Affero General Public License as published by
*   the Free Software Foundation, either version 3 of the License, or
*   (at your option) any later version.

*   This program is distributed in the hope that it will be useful,
*   but WITHOUT ANY WARRANTY; without even the implied warranty of
*   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*   GNU Affero General Public License for more details.

*   You should have received a copy of the GNU Affero General Public License
*   along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use autonomi::client::key_derivation::{DerivationIndex, MainSecretKey};
use autonomi::SecretKey;

/// Each mutable type has an address derived from three pieces of information:
/// - an owner secret
/// - a derivation index (which prevents clashes between types)
/// - a name (which allows the same owner to create more than one instance of each type)
///
/// The owner secret is supplied to the API and used with the derivation index
/// to derive an owner secret per type (see derive_owner_secret_for_type())
/// The owner secret and name are provided to the API by an app, whereas
/// the following derivation indices are fixed by dweb. Other libraries
/// can use the same approach with the same derivation indices in order
/// to allow different apps to access the same objects providing they
/// have the owner secret and know the name used to create the object.
///
/// Derivation indices for each mutable Autonomi type:
/// Note: the string must be exactly 32 bytes long and different from all other indices
pub const POINTER_DERIVATION_INDEX: &str = "Pointer derivatation index      ";
pub const GRAPHENTRY_DERIVATION_INDEX: &str = "GraphEntry derivatation index   ";
pub const SCRATCHPAD_DERIVATION_INDEX: &str = "Scratchpad derivatation index   ";
// TODO see autonomi::access::keys and notes in Zim
// pub const VAULT_DERIVATION_INDEX: &str = "Vault derivatation index        ";
// pub const REGISTER_DERIVATION_INDEX: &str = "Register derivatation index     ";
///
/// Derivation indices for each mutable Dweb type:
/// Note: A dweb History doesn't have its own derivation index. Instead because it
/// is a generic type and uses the trove_type() to ensure each specific History
/// type has a separate derivation index.
pub const HISTORY_POINTER_DERIVATION_INDEX: &str = "History Pointer derivatatn. idx";

/// Get the main secret key for all Pointers belonging to an owner
pub fn pointer_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
    derive_type_owner_secret(owner_secret, POINTER_DERIVATION_INDEX)
}

/// Get the main secret key for all GraphEntry objects belonging to an owner
pub fn graphentry_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
    derive_type_owner_secret(owner_secret, GRAPHENTRY_DERIVATION_INDEX)
}

/// Get the main secret key for all Scratchpads belonging to an owner
pub fn scratchpad_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
    derive_type_owner_secret(owner_secret, SCRATCHPAD_DERIVATION_INDEX)
}

// /// Get the main secret key for all Vaults belonging to an owner
// /// TODO see autonomi::access::keys and notes in Zim
// pub fn vault_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
//     derive_type_owner_secret(owner_secret, VAULT_DERIVATION_INDEX)
// }

// /// Get the main secret key for all Register belonging to an owner
// /// TODO see autonomi::access::keys and notes in Zim
// pub fn register_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
//     derive_type_owner_secret(owner_secret, REGISTER_DERIVATION_INDEX)
// }

/// Use the secret for a type to obtain the owner secret for creating or updating a named object of that type
pub fn derive_named_object_secret(type_owner_secret: SecretKey, name: Option<String>) -> SecretKey {
    if name.is_some() {
        type_owner_secret.derive_child(name.unwrap().as_bytes())
    } else {
        type_owner_secret
    }
}

/// Derive a secret key for a type using its derivation index
pub(crate) fn derive_type_owner_secret(
    main_owner_secret: SecretKey,
    derivation_index: &str,
) -> SecretKey {
    let derivation_index: [u8; 32] = derivation_index.as_bytes().try_into().unwrap();
    MainSecretKey::new(main_owner_secret.clone())
        .derive_key(&DerivationIndex::from_bytes(derivation_index))
        .into()
}
