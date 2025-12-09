use log;
use std::thread;

const MAX_THREADS: usize = 16;

/// Get the max number of threads to use for parallel processing.
pub fn get_total_threads() -> usize {
    match thread::available_parallelism() {
        Ok(i) => i.get(),
        Err(e) => {
            log::warn!("Unable to obtain available parellelism: {}", e);
            log::warn!("Defaulting to 1 thread");
            1
        }
    }
}

/// Apply a function in parallel to the given items. Results will be returned in the
/// same order as the inputs.
pub fn parallel_apply<T: Clone + Send, U: Send>(mut items: Vec<T>, mapping: fn(T) -> U) -> Vec<U> {
    if items.len() == 0 {
        return Vec::new();
    }
    let num_threads = get_total_threads().min(items.len()).min(MAX_THREADS);
    log::debug!("Mapping with {} threads", num_threads);

    if num_threads == 1 {
        // Single thread - run simple mapping
        let mut out = Vec::with_capacity(items.len());
        for elem in items {
            out.push(mapping(elem));
        }
        return out;
    }

    let chunk_size = items.len() / num_threads + 1;
    let mut input_groups: Vec<Vec<T>> = Vec::with_capacity(num_threads);
    for chunk in items.chunks_mut(chunk_size) {
        input_groups.push(chunk.into());
    }

    let mut output = Vec::with_capacity(items.len());

    thread::scope(|s| {
        let mut handles = Vec::with_capacity(num_threads);
        for elems in input_groups {
            handles.push(s.spawn(move || {
                let mut outputs = Vec::with_capacity(elems.len());
                for elem in elems {
                    outputs.push(mapping(elem));
                }
                outputs
            }));
        }

        for h in handles {
            let mut res = h.join().unwrap();
            output.append(&mut res);
        }
    });

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_apply() {
        let total_par = dbg!(get_total_threads());
        assert!(total_par > 1);
        let my_inputs: Vec<usize> = (0..total_par * 4).collect();

        let mapped = parallel_apply(my_inputs.clone(), |num| num + 1);

        assert_eq!(mapped.len(), my_inputs.len());
        for (idx, &item) in mapped.iter().enumerate() {
            assert_eq!(idx, item - 1)
        }
    }
}
