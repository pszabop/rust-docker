use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::collections::HashSet;

const NUM_ZYGOTES: usize = 50;
const NUM_ALLELES: usize = 10; // Number of alleles
const NUM_SIMULATIONS: usize = 1; // Run 100 simulations for averaging
const SELECTION_PRESSURE: f64 = 0.10; // Selection pressure coefficient

fn calculate_fitness(alleles: &HashSet<usize>) -> f64 {
    // Calculate fitness based on the alleles present
    // For simplicity, assume each allele contributes equally to fitness
    alleles.len() as f64 * SELECTION_PRESSURE
}
fn initialize_population() -> Vec<HashSet<usize>> {
    let mut rng = thread_rng();
    let mut alleles: Vec<usize> = (0..NUM_ALLELES).collect();
    alleles.shuffle(&mut rng);

    let mut population = Vec::new();
    for i in 0..NUM_ZYGOTES {
        let zygote = if i < NUM_ALLELES {
            let mut zygote = HashSet::new();
            zygote.insert(alleles[i]);
            zygote
        } else {
            HashSet::new()
        };
        population.push(zygote);
    }
    println!("Initial population: {:?}", population);
    population
}


fn simulate_generation(population: &mut Vec<HashSet<usize>>) {
    let mut rng = thread_rng();
    let mut next_generation = Vec::new();
    let mut max_alleles = 0;
    let mut offspring_total_alleles:f64 = 0.0;
    let mut most_diverse_offspring = HashSet::new();

    // Shuffle the population to ensure random pairing without replacement
    let mut shuffled_population = population.clone();
    shuffled_population.shuffle(&mut rng);

    // Iterate over the shuffled population in pairs
    for pair in shuffled_population.chunks(2) {
        //println!("Pair: {:?}", pair);
        if pair.len() == 2 {
            let (parent1, parent2) = (&pair[0], &pair[1]);
            let fitness1 = calculate_fitness(&parent1);
            let fitness2 = calculate_fitness(&parent2);

            // Determine the number of offspring based on fitness
            let num_offspring = (((fitness1 + fitness2) * 4.0) +1.5).round() as usize;

            // Generate offspring for parent1
            for _ in 0..num_offspring {
                let mut offspring = HashSet::new();
                let alleles_set: HashSet<_> = parent1.union(parent2).cloned().collect();

                // Randomly select alleles to form a unique combination
                for allele in alleles_set {
                    if parent1.contains(&allele) && parent2.contains(&allele) {
                        // If the allele is present in both parents, include it in the offspring
                        offspring.insert(allele);
                    } else if rng.gen_bool(0.5) {
                        // If the allele is present in only one parent, include it with 50% probability
                        offspring.insert(allele);
                    }
                }

                offspring_total_alleles = offspring_total_alleles + offspring.len() as f64;

                // Update the most diverse offspring if this one has more alleles
                if offspring.len() > max_alleles {
                    //println!("MAX ALLELES Offspring: {:?}", offspring);
                    max_alleles = offspring.len();
                    most_diverse_offspring = offspring.clone();
                }

                next_generation.push(offspring);
            }

        }
    }

    // Print the most diverse offspring of this generation
    println!("Most diverse offspring in this generation: {:?}, average {}", most_diverse_offspring, offspring_total_alleles/(next_generation.len() as f64));

    // Update the population for the next generation
    *population = next_generation;
}

fn run_simulation() -> usize {
    let mut population = initialize_population();

    let mut generations = 0;

    loop {
        generations += 1;

        // Print the generation number and population size
        println!("Generation {}: Population size = {}", generations, population.len());

        // Simulate a generation
        simulate_generation(&mut population);

        // Check if any zygote has all alleles
        if population.iter().any(|zygote| zygote.len() == NUM_ALLELES) {
            break;
        }
        if generations > 30 {
            break;
        }
    }

    generations
}

fn main() {
    let mut total_generations = 0;

    for _ in 0..NUM_SIMULATIONS {
        total_generations += run_simulation();
    }

    let average_generations = total_generations / NUM_SIMULATIONS;
    println!("Average generations to fixation: {}", average_generations);
}
