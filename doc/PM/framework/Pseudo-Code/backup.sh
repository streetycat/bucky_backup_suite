source_url = python3 source.py
target_url = python3 target.py

python3 engine.py

source_id = engine.register_source(source_url)
target_id = engine.register_target(target_url)

task = engine.create_task(source_id, '/photos/', target_id, '')

source_preserved_state_id = task.preserve_source()

checkpoint = task.prepare_checkpoint(source_preserved_state_id, false)

checkpoint.transfer()

while !checkpoint.is_done():
    time.sleep(1)


