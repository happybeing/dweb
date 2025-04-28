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
pub const PRIVATE_SCRATCHPAD_DERIVATION_INDEX: &str = "PublicScratchpad derivn. index  ";
pub const PUBLIC_SCRATCHPAD_DERIVATION_INDEX: &str = "PrivateScratchpad derivn. index ";
// TODO see autonomi::access::keys and notes in Zim
// pub const VAULT_DERIVATION_INDEX: &str = "Vault derivatation index        ";
// pub const REGISTER_DERIVATION_INDEX: &str = "Register derivatation index     ";
///
/// Derivation indices for each mutable Dweb type:
/// Note: A dweb History doesn't have its own derivation index. Instead because it
/// is a generic type and uses the trove_type() to ensure each specific History
/// type has a separate derivation index.
pub const HISTORY_POINTER_DERIVATION_INDEX: &str = "History Pointer derivatatn. indx";

// /// Get the main secret key for all Pointers belonging to an owner
// pub fn pointer_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
//     derive_type_owner_secret(owner_secret, POINTER_DERIVATION_INDEX)
// }

// /// Get the main secret key for all GraphEntry objects belonging to an owner
// pub fn graphentry_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
//     derive_type_owner_secret(owner_secret, GRAPHENTRY_DERIVATION_INDEX)
// }

// /// Get the main secret key for all Scratchpads belonging to an owner
// pub fn scratchpad_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
//     derive_type_owner_secret(owner_secret, SCRATCHPAD_DERIVATION_INDEX)
// }

// // /// Get the main secret key for all Vaults belonging to an owner
// // /// TODO see autonomi::access::keys and notes in Zim
// // pub fn vault_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
// //     derive_type_owner_secret(owner_secret, VAULT_DERIVATION_INDEX)
// // }

// /// Get the main secret key for all Register belonging to an owner
// /// TODO see autonomi::access::keys and notes in Zim
// pub fn register_secret_key_from_owner(owner_secret: SecretKey) -> SecretKey {
//     derive_type_owner_secret(owner_secret, REGISTER_DERIVATION_INDEX)
// }

/// Derive the object owner secret when creating a mutable data object (e.g. Pointer or Scratchpad)
///
/// The owner_secret for a mutable objecct is based on the dweb derivation key for the type or a supplied string, an
/// optional object name and optional app identifying strings from request headers.
///
/// If all mutable objects were created with the owner_secret they would all have the same address and only one
/// would be permitted. To allow multiple objects to be created, the secret used to create them can be derived
/// from from the owner secret using one or more derivation indexes (32 byte sequences) or strings (such as an app identifier
/// and object name).
pub fn derive_named_object_secret(
    // Main owner secret
    owner_secret: SecretKey,
    // The dweb derivation index for the type
    type_derivation_index: &str,
    // Optional override of type_derivation_index (e.g. from request headers)
    supplied_derivation_index: &Option<[u8; 32]>,
    // Optional app ID to tie ownership of data to this app
    app_id: Option<String>,
    // Optional object name to differentiate objects of a type that are otherwise within the same scope
    supplied_name: Option<String>,
) -> SecretKey {
    let object_type_derivation_index =
        supplied_derivation_index.unwrap_or(type_derivation_index.as_bytes().try_into().unwrap());

    let mut type_owner_secret: SecretKey = MainSecretKey::new(owner_secret)
        .derive_key(&DerivationIndex::from_bytes(object_type_derivation_index))
        .into();

    if app_id.is_some() {
        type_owner_secret = type_owner_secret.derive_child(app_id.unwrap().as_bytes());
    };

    if supplied_name.is_some() {
        type_owner_secret.derive_child(supplied_name.unwrap().as_bytes())
    } else {
        type_owner_secret
    }
}

/// Derive the object owner secret based on the dweb derivation key for the type or a supplied str, and an
/// optional object name
pub fn derive_named_object_secret_old(
    owner_secret: SecretKey,
    type_derivation_index: &str,
    supplied_derivation_index: &Option<[u8; 32]>,
    supplied_name: Option<String>,
) -> SecretKey {
    let object_type_derivation_index =
        supplied_derivation_index.unwrap_or(type_derivation_index.as_bytes().try_into().unwrap());

    let type_owner_secret: SecretKey = MainSecretKey::new(owner_secret)
        .derive_key(&DerivationIndex::from_bytes(object_type_derivation_index))
        .into();

    if supplied_name.is_some() {
        type_owner_secret.derive_child(supplied_name.unwrap().as_bytes())
    } else {
        type_owner_secret
    }
}
