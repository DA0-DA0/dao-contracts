use std::collections::{HashMap, HashSet};

use cosmwasm_std::Deps;
use prost_reflect::{
    prost::Message,
    prost_types::{FileDescriptorProto, FileDescriptorSet},
};
use prost_types::{field_descriptor_proto, FieldDescriptorProto};

use crate::{
    state::{FILES, MESSAGES},
    ContractError,
};

/// Create a file descriptor set that contains the exact files with the exact
/// messages/enums needed to decode a set of messages.
pub fn create_file_descriptor_set_for_messages(
    deps: &Deps,
    messages: &[String],
) -> Result<FileDescriptorSet, ContractError> {
    let mut file_descriptor_set = FileDescriptorSet::default();
    // Map of file name to index in file descriptor set.
    let mut file_map = HashMap::<String, usize>::new();
    // Cache of loaded file descriptors.
    let mut file_cache = HashMap::<String, FileDescriptorProto>::new();

    for message in messages {
        let file_name = MESSAGES
            .may_load(deps.storage, message.clone())?
            .ok_or_else(|| ContractError::ProtobufMessageNotFound {
                message: message.to_string(),
            })?;

        // Load file and its dependency tree into cache.
        load_file_into_cache(deps, &file_name, &mut file_cache)?;

        // Add the message and its dependency tree to the file descriptor set.
        add_dependency(
            DependencyToLoad::new(&file_name, message, true),
            &mut file_descriptor_set,
            &mut file_map,
            &file_cache,
        )?;
    }

    Ok(file_descriptor_set)
}

/// Load a file and its dependency tree into the cache.
fn load_file_into_cache(
    deps: &Deps,
    initial_file_name: &str,
    file_cache: &mut HashMap<String, FileDescriptorProto>,
) -> Result<(), ContractError> {
    // Use iterative approach with better performance than recursion.
    let mut stack = vec![initial_file_name.to_string()];

    while let Some(file_name) = stack.pop() {
        // Skip if already in cache.
        if file_cache.contains_key(&file_name) {
            continue;
        }

        // Load file and its dependency tree into cache.
        let file_data = FILES.load(deps.storage, file_name.clone())?;
        let file_descriptor = FileDescriptorProto::decode(file_data.as_slice())?;

        // Add dependencies to stack for processing if not in cache.
        //
        // Protobuf files are guaranteed to be acyclic, so we'll never infinite
        // loop. And the overhead of hashing with a HashSet is not worth it
        // compared to the redundant loops that are skipped by the check at the
        // beginning when dependencies are shared between files. The important
        // thing is that we don't load a file from storage more than once.
        for dep_name in &file_descriptor.dependency {
            if !file_cache.contains_key(dep_name) {
                stack.push(dep_name.clone());
            }
        }

        // Add file to cache.
        file_cache.insert(file_name.clone(), file_descriptor);
    }

    Ok(())
}

/// A dependency to load, which is either a message or enum descriptor.
struct DependencyToLoad {
    /// File name that contains the dependency.
    file_name: String,
    /// Package of the dependency.
    package: String,
    /// Name of the dependency.
    name: String,
    /// Full name of the dependency.
    full_name: String,
    /// Whether the dependency is a message (otherwise it is an enum).
    is_message: bool,
}

impl DependencyToLoad {
    fn new(file_name: &str, full_name: &str, is_message: bool) -> Self {
        if let Some(last_dot_pos) = full_name.rfind('.') {
            let package = full_name[..last_dot_pos].to_string();
            let name = full_name[last_dot_pos + 1..].to_string();
            Self {
                file_name: file_name.to_string(),
                package,
                name,
                full_name: full_name.to_string(),
                is_message,
            }
        } else {
            Self {
                file_name: file_name.to_string(),
                package: String::new(),
                name: full_name.to_string(),
                full_name: full_name.to_string(),
                is_message,
            }
        }
    }

    /// Create a dependency with a full type name that may be in one of the
    /// provided files. If not found, return `None`.
    fn new_from_files(
        potential_files: &[&FileDescriptorProto],
        full_name: &str,
        is_message: bool,
    ) -> Result<Option<Self>, ContractError> {
        let mut dep = Self::new("", full_name, is_message);

        // Find dependency containing this type. If found, set the file name and
        // return it.
        let dependency_source = dep.find_file_descriptor(potential_files);
        if let Some(dependency_source) = dependency_source {
            dep.file_name = dependency_source
                .name
                .as_ref()
                .ok_or(ContractError::InternalError {
                    msg: "dependency source name not found".to_string(),
                })?
                .clone();

            Ok(Some(dep))
        } else {
            Ok(None)
        }
    }

    /// Find the file descriptor containing the message or enum from a set of
    /// file descriptors.
    fn find_file_descriptor<'b>(
        &self,
        file_descriptor: &'b [&'b FileDescriptorProto],
    ) -> Option<&'b &'b FileDescriptorProto> {
        file_descriptor.iter().find(|f| {
            f.package.as_ref() == Some(&self.package)
                && ((self.is_message
                    && f.message_type
                        .iter()
                        .any(|m| m.name.as_ref() == Some(&self.name)))
                    || (!self.is_message
                        && f.enum_type
                            .iter()
                            .any(|e| e.name.as_ref() == Some(&self.name))))
        })
    }

    /// Get the fields of the message descriptor, if found. Enums have no
    /// fields.
    fn get_message_descriptor_fields<'a>(
        &self,
        file: &'a FileDescriptorProto,
    ) -> Option<Vec<&'a FieldDescriptorProto>> {
        if !self.is_message {
            return Some(vec![]);
        }

        file.message_type
            .iter()
            .find(|m| m.name.as_ref() == Some(&self.name))
            .map(|m| m.field.iter().collect::<Vec<_>>())
    }

    /// Add the message/enum to the file descriptor.
    fn add_to_file(
        &self,
        file: &mut FileDescriptorProto,
        from: &FileDescriptorProto,
    ) -> Result<(), ContractError> {
        if self.is_message {
            file.message_type.push(
                from.message_type
                    .iter()
                    .find(|m| m.name.as_ref() == Some(&self.name))
                    .ok_or(ContractError::InternalError {
                        msg: format!("message descriptor not found for {}", self.full_name),
                    })?
                    .clone(),
            );
        } else {
            file.enum_type.push(
                from.enum_type
                    .iter()
                    .find(|e| e.name.as_ref() == Some(&self.name))
                    .ok_or(ContractError::InternalError {
                        msg: format!("enum descriptor not found for {}", self.full_name),
                    })?
                    .clone(),
            );
        }

        Ok(())
    }
}

/// Find all messages/enums referenced by a protobuf message (recursively) and
/// add just the necessary messages/enums to the file descriptor set.
fn add_dependency(
    // Dependency to load.
    dependency: DependencyToLoad,
    // File descriptor set to add dependencies to.
    file_descriptor_set: &mut FileDescriptorSet,
    // Map of file name to index in file descriptor set.
    file_map: &mut HashMap<String, usize>,
    // Cache of loaded file descriptors.
    file_cache: &HashMap<String, FileDescriptorProto>,
) -> Result<(), ContractError> {
    // Caller should have already loaded the file into cache.
    let source_file =
        file_cache
            .get(&dependency.file_name)
            .ok_or_else(|| ContractError::InternalError {
                msg: "source file descriptor not found".to_string(),
            })?;

    // Get message descriptor fields for this dependency.
    let fields = dependency
        .get_message_descriptor_fields(source_file)
        .ok_or_else(|| ContractError::InternalError {
            msg: "message descriptor fields not found".to_string(),
        })?;

    // Get all files that may contain referenced messages/enums.
    let mut potential_sources = Vec::new();
    for dep_name in &source_file.dependency {
        let dep_file = file_cache
            .get(dep_name)
            .ok_or_else(|| ContractError::InternalError {
                msg: "dependency file descriptor not found".to_string(),
            })?;
        potential_sources.push(dep_file);
    }
    // Add self to beginning of potential sources.
    potential_sources.insert(0, source_file);

    let mut used_dependencies = HashSet::<String>::new();

    for field in fields {
        let is_message = match field.r#type {
            Some(t) if t == field_descriptor_proto::Type::Message as i32 => true,
            Some(t) if t == field_descriptor_proto::Type::Enum as i32 => false,
            _ => continue, // Early continue for non-message/enum types
        };

        // If has full type name set that starts with `.` prefix, this is a
        // message/enum reference. Strip the prefix to get the full type name.
        if let Some(full_type_name) = field.type_name.as_ref().and_then(|s| s.strip_prefix('.')) {
            let found_dependency =
                DependencyToLoad::new_from_files(&potential_sources, full_type_name, is_message)?;

            if let Some(dep) = found_dependency {
                // Mark dependency as used so we keep it in the set.
                if !used_dependencies.contains(&dep.file_name) {
                    used_dependencies.insert(dep.file_name.clone());
                }

                add_dependency(dep, file_descriptor_set, file_map, file_cache)?;
            }
        }
    }

    // Add dependency to file in map / create new file if not found.
    if let Some(&file_index) = file_map.get(&dependency.file_name) {
        // File already exists in the set, add dependency to it.
        let file = &mut file_descriptor_set.file[file_index];
        dependency.add_to_file(file, source_file)?;
    } else {
        // Create new file descriptor.
        let mut new_file = FileDescriptorProto {
            name: Some(dependency.file_name.clone()),
            package: source_file.package.clone(),
            // Only include dependencies that have been used.
            dependency: source_file
                .dependency
                .clone()
                .into_iter()
                .filter(|d| used_dependencies.contains(d))
                .collect(),
            public_dependency: vec![],
            weak_dependency: vec![],
            message_type: vec![],
            enum_type: vec![],
            service: vec![],
            extension: vec![],
            options: None,
            source_code_info: None,
            syntax: source_file.syntax.clone(),
        };
        // Add dependency to new file.
        dependency.add_to_file(&mut new_file, source_file)?;
        // Add new file to set.
        let file_index = file_descriptor_set.file.len();
        file_descriptor_set.file.push(new_file);

        // Store index of new file in set for quick lookup later.
        file_map.insert(dependency.file_name, file_index);
    }

    Ok(())
}
