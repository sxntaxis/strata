from pathlib import Path

path = Path("src/storage.rs")
text = path.read_text()
old = """        for payload in payloads.iter().cloned() {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                atomic_write(&path, &payload)
            }));
        }
"""
new = """        for payload in payloads.clone() {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                atomic_write(&path, &payload)
            }));
        }
"""
count = text.count(old)
if count != 1:
    raise SystemExit(f"race-test ownership marker: expected one, found {count}")
path.write_text(text.replace(old, new, 1))
