# Pseudo-Code

```rust
engine.preserve_source_state(
    task_friendly_name,
    task_uuid,
    (source_type, source_interface_url, source_session_id),
    param,
    (target_service_type, target_service_name),
    is_delta
    ) -> SourceStateId {
        // save original stat of data source, maybe we should reset the stat to original;
        // so, we should not write anything in this interface.
        let origin_stat = data_source.read_data_source_original_stat();
        let state_id = local_storage.save_original_stat(task_uuid, origin_stat);

        let prepared_stat = data_source.prepare_data_source(origin_stat);
        local_storage.save_prepared_stat(state_id, prepared_stat);
        state_id
    }
```

```rust
// Prepare the metadata before transfering the data to storage.
// metadata should contain:
// 1. data items should transfering,
// 2. prev-checkpoint the new checkpoint depending on,
// 3. measure the space size, it should be larger the consume really.
// 4. do not write anything in this interface, maybe it will not recover the state when it abort unexpected.
engine.prepare_checkpoint(
    state_id: SourceStateId
    ) -> CheckPointMeta {
        let {
                task_friendly_name,
                task_uuid,
                (source_type, source_interface_url, source_session_id),
                param,
                (target_service_type, target_service_name),
                is_delta
            } = local_storage.read_preserved_state(state_id);

        let source = source_from_url(source_interface_url, task_uuid, source_session_id);

        let target = select_target(target_service_type, target_service);
        let (last_complete_checkpoint, last_checkpoint, transfering_checkpoints) = engine.get_last_checkpoint(task_uuid);
        let checkpoint_version = engine.generate_checkpoint_version(task_uuid);
        let mut meta_builder = MetaBuilder::new()
            .task_uuid(task_uuid)
            .task_friendly(task_friendly_name)
            .target_service(target_service_type, target_service_name)
            .checkpoint(checkpoint_version)
            .source(source_type, souce_interface_url, source_session_id);

        let prev_checkpoints = if is_delta {
            meta_builder.prev_checkpoint(last_complete_checkpoint.version);

            let mut prev_checkpoints = vec![last_complete_checkpoint];
            let mut cur_checkpoints = &last_complete_checkpoint;
            while let Some(prev_checkpoint_version) = cur_checkpoints.prev_checkpoint() {
                let prev_checkpoint = engine.get_checkpoint_by_version(prev_checkpoint_version);
                prev_checkpoints.push(prev_checkpoint);
            }
        } else {
            vec![]
        };

        while let Some(item) = source.get_next_item() {
            // * item may be a file/directory/link, we should process it separately.
            if is_delta {
                let last_stat = prev_checkpoints.get_stat(item.path);
                if last_stat.is_none() {
                    meta_builder.add_item(item);
                } else if !last_stat.is_same_as(item.stat) {
                    let last_content = target.read(item.path);
                    let new_content = source.read(item.path);
                    let diff = engine.diff(new_content, last_content);
                    if diff.is_some() {
                        meta_builder.add_item(diff);
                    } else {
                        meta_builder.add_item(item);
                    }
                }
            } else {
                meta_builder.add_item(item);
            }
        }

        if is_delta {
            while let Some(item) = prev_checkpoints.get_next_item() {
                if !source.is_exists(item.path) {
                    meta_builder.add_log(Log::Remove(item.path));
                }
            }
        }

        meta_builder.build()
}
```

-   The definition of `CheckPointMeta` is in [this page](./meta.md).

```rust
engine.start_transfer_checkpoint(
    meta: CheckPointMeta,
    uncomplete_strategy
    ) {
        let target = select_target(meta.target_service_type, meta.target_service);
        let (last_complete_checkpoint, last_checkpoint, transfering_checkpoints) = egnine.get_last_checkpoint(meta.task_unique_name);
        let mut abort_checkpoints = vec![];

        for transfering_checkpoint in transfering_checkpoints {
            if uncomplete_strategy.check_abort(transfering_checkpoint, last_complete_checkpoint, REASON_TRANSFER_NEW_CHECKPOINT) {
                // abort it when new checkpoint prepared.
                abort_checkpoints.push(transfering_checkpoint);
            }
        }

        local_meta_storage.begin_checkpoint(meta, abort_checkpoints);

        let target_filled_meta = target.fill_target_meta(meta);

        local_meta_storage.save_target_filled_meta(target_filled_meta);

        target.transfer_checkpoint(target_filled_meta);

        for abort_checkpoint in abort_checkpoints {
            abort_checkpoint.abort();
            // remove when it's aborted by target
            local_meta_storage.abort(abort_checkpoint);
            target.abort(abort_checkpoint);
        }
}
```

```rust
local_file_source.read_data_source_original_stat(self) {
    if fs.snapshot_supported() {
        return generate_snapshot_uuid();
    } else self.readonly_allow() {
        return read_stat_of_local_files();
    } else {
        return None;
    }
}

local_file_source.prepare_data_source(origin_stat) {
    if origin_stat.is_snapshot() {
        let snapshot = create_snapshot_by_id(origin_stat.snapshot_uuid);
        return snapshot;
    } else self.is_readonly() {
        let changed_stats = set_readonly_for_all_files();
        return changed_stats;
    } else {
        return None;
    }
}

local_file_source.check_point_complete(origin_stat, prepared_stat) {
    if origin_stat.is_snapshot() {
        remove_snapshot(prepared_stat.snapshot);
    } else self.is_readonly() {
        reset_stats_for_all_files(prepared_stat.changed_stats);
    } else {
        // nothing
    }
}
```

```rust
// The following is just a schematic representation of the process for the `target` part.
// Due to space management and interface differences, I do not want to create a highly abstract unified structure in the `target` module, as that could be quite challenging.
// My current idea is to define only a few coarse-grained interfaces, pass in the metadata, and let `target` allocate and manage the storage space according to its own needs.
// The specific data is then read into the corresponding storage space through the read interface provided by the `engine` module.
target.fill_target_meta(meta) {
    // ** attention: reentry
    let mut spaces = vec![];
    for item in meta.items() {
        let space = alloc_space(item);
        spaces.push(space);
    }

    meta.add_space(spaces);
}

target.transfer_checkpoint(target_filled_meta) {
    for space in target_filled_meta.spaces {
        space.write_header(make_header(space));
        for item in space.items() {
            let source_reader = source_reader_from_url(meta, meta.get_item(item));
            let chunk = source_reader.read_chunk(item.path, item.offset, item.length, item.meta);
            let chunk = crypto(chunk);
            space.write_chunk(chunk);
        }
        space.write_meta(crypto(space.meta));

        engine.pre_transfer(space.meta);

        space.transfer();

        engine.transfer_complete(space);
    }

    // update status to complete;
    // clear temporary storage
    engine.check_point_complete(meta);
}
```

```rust
target.on_abort(checkpoint) {
    let spaces = find_spaces(checkpoint);

    for space in spaces {
        local_storage.pre_free_space(space);
        space.free();
        local_storage.free_space(space);
    }

    // remove the checkpoint
    engine.check_point_space_free(checkpoint);
}
```

```rust
engine.export() {
    let tasks = local_meta_storage.export();

    let mut toml = Toml.create();

    for t in tasks {
        toml.write_section(t.unique_task_name);
        for checkpoint in t. checkpoints {
            toml.write_section(checkpoint.version);
            toml.write_attribute(checkpoint);
            let target = select_target(checkpoint.target_service);
            let content = target.export(); // export valid attributes only
            toml.write_target_attributes(content);
        }
    }
}
```
