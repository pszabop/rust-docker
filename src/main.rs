use rand::Rng;

// Simulate the probability of fixation for each allele carrier in the population
fn calculate_probability(allele_carriers: &Vec<u32>, k: u32, N: usize) -> f64 {
    let carriers_with_all_alleles = allele_carriers.iter().filter(|&&x| x == k).count();
    carriers_with_all_alleles as f64 / N as f64
}

// Simulate the fixation process with selection pressure
fn simulate_fixation_with_selection(N: usize, k: u32, initial_alleles: usize, s: f64) -> usize {
    let mut rng = rand::thread_rng();
    let mut allele_carriers = vec![1; initial_alleles]; // Initially, 10 zygotes with 1 allele each
    allele_carriers.extend(vec![0; N - initial_alleles]); // Fill the rest with 0 alleles
    let mut generations = 0;

    // Simulate the generations until one zygote has all alleles
    while calculate_probability(&allele_carriers, k, N) < 0.5 {
        generations += 1;
        let mut new_allele_carriers = allele_carriers.clone();

        // Reproduction and recombination with selection pressure
        for i in 0..N {
            if allele_carriers[i] == k {
                continue; // Skip if the zygote already has all alleles
            }
            for j in (i + 1)..N {
                if allele_carriers[j] == k {
                    continue; // Skip if the zygote already has all alleles
                }

                // Simulate recombination between zygotes in each generation with selection pressure
                //let fitness_i = 1.0 + s * allele_carriers[i] as f64; // Selection pressure on i
                //let fitness_j = 1.0 + s * allele_carriers[j] as f64; // Selection pressure on j
                let fitness_i = 1.0 + (s * allele_carriers[i] as f64); // Selection pressure on i
                let fitness_j = 1.0 + (s * allele_carriers[j] as f64); // Selection pressure on j


                let recombination_prob = 1.0 / N as f64;
                let combined_fitness_prob = fitness_i * fitness_j / (fitness_i + fitness_j);

                if rng.gen::<f64>() < recombination_prob * combined_fitness_prob {
                    let combined_alleles = (allele_carriers[i] + allele_carriers[j]).min(k);
                    new_allele_carriers[i] = combined_alleles;
                    new_allele_carriers[j] = combined_alleles;
                }
            }
        }

        allele_carriers = new_allele_carriers;

        // Check if fixation has been achieved (at least one zygote with all alleles)
        if calculate_probability(&allele_carriers, k, N) >= 0.5 {
            break;
        }
    }

    generations
}

fn main() {
    let N = 50; // Population size
    let k = 10; // Number of alleles
    let initial_alleles = 10; // Number of zygotes initially carrying one allele each
    let s = 0.1; // Selection coefficient (how much selection favors certain alleles)
    let trials = 100; // Number of Monte Carlo trials

    let mut total_generations = 0;

    // Run Monte Carlo simulations with selection and accumulate results
    for _ in 0..trials {
        total_generations += simulate_fixation_with_selection(N, k, initial_alleles, s);
    }

    // Calculate average generations for fixation
    let average_generations = total_generations as f64 / trials as f64;
    println!("Average generations for fixation with selection: {:.2}", average_generations);
}
