//! Genetics calculation engine.
//!
//! Implements Hardy-Weinberg equilibrium, Punnett squares, and
//! inheritance risk calculations.

/// Result of a Hardy-Weinberg equilibrium calculation.
#[derive(Debug)]
pub struct HardyWeinbergResult {
    pub q: f64,                          // Recessive allele frequency
    pub p: f64,                          // Dominant allele frequency
    pub q_squared: f64,                  // Homozygous recessive (affected)
    pub two_pq: f64,                     // Heterozygous (carriers)
    pub p_squared: f64,                  // Homozygous dominant
    pub carrier_ratio: f64,              // Ratio q / q^2 (carriers per affected)
    pub het_x_het_affected_risk: f64,    // Risk child is affected (carrier x carrier)
    pub het_x_het_carrier_risk: f64,     // Risk child is carrier (carrier x carrier)
    pub het_x_het_unaffected_risk: f64,  // Risk child is unaffected (carrier x carrier)
}

/// Perform Hardy-Weinberg equilibrium calculation.
///
/// # Arguments
/// * `affected` — Number of affected individuals (homozygous recessive)
/// * `population` — Total population size
pub fn hardy_weinberg(affected: f64, population: f64) -> Result<HardyWeinbergResult, String> {
    if population <= 0.0 {
        return Err("Population must be greater than zero.".to_string());
    }
    if affected < 0.0 || affected > population {
        return Err("Affected count must be between 0 and population.".to_string());
    }

    let q_squared = affected / population;
    let q = q_squared.sqrt();
    let p = 1.0 - q;

    let two_pq = 2.0 * p * q;
    let p_squared = p * p;

    // Risk calculations for carrier x carrier cross
    let het_x_het_affected_risk = 0.25;    // 1/4 probability
    let het_x_het_carrier_risk = 0.50;     // 2/4 probability
    let het_x_het_unaffected_risk = 0.25;  // 1/4 probability

    Ok(HardyWeinbergResult {
        q,
        p,
        q_squared,
        two_pq,
        p_squared,
        carrier_ratio: if q_squared > 0.0 { two_pq / q_squared } else { 0.0 },
        het_x_het_affected_risk,
        het_x_het_carrier_risk,
        het_x_het_unaffected_risk,
    })
}

/// Format a Hardy-Weinberg result as a human-readable string.
pub fn format_hardy_weinberg(r: &HardyWeinbergResult, affected: f64, population: f64) -> String {
    format!(
        "🧬 **Equilíbrio de Hardy-Weinberg**\n\
        População total: {:.0} | Afetados: {:.0}\n\
        \n\
        📊 **Resumo Estatístico:**\n\
        • **Afetados (qq):** {:.3}% (aprox. {:.0} indivíduos)\n\
        • **Portadores (Hq):** {:.3}% (aprox. {:.0} indivíduos)\n\
        • **Normais (HH):** {:.3}% (aprox. {:.0} indivíduos)\n\
        \n\
        🧬 **Frequências Alélicas:**\n\
        • Alelo recessivo (q): {:.5}\n\
        • Alelo dominante (p): {:.5}\n\
        \n\
        💡 **Curiosidades:**\n\
        • Para cada afetado, existem cerca de **{:.1} portadores** na população.\n\
        \n\
        👶 **Riscos de Transmissão (Casal Portador Hq x Hq):**\n\
        • Risco de filho afetado (qq): 25%\n\
        • Risco de filho portador (Hq): 50%\n\
        • Risco de filho não afetado (HH): 25%",
        population, affected,
        r.q_squared * 100.0, affected,
        r.two_pq * 100.0, r.two_pq * population,
        r.p_squared * 100.0, r.p_squared * population,
        r.q,
        r.p,
        r.carrier_ratio,
    )
}

/// Result of a Punnett square cross.
#[derive(Debug)]
pub struct PunnettResult {
    pub offspring: Vec<String>,    // All 4 genotype outcomes
    pub genotype_counts: std::collections::HashMap<String, usize>,
    pub phenotype_affected: f64,   // Proportion affected (homozygous recessive)
    pub phenotype_carrier: f64,    // Proportion carriers
    pub phenotype_unaffected: f64, // Proportion fully unaffected
}

/// Perform a Punnett square cross for a single gene with two alleles.
///
/// Alleles are represented as: uppercase = dominant, lowercase = recessive.
/// Genotypes: "AA", "Aa", "aA", "aa" — all two-character inputs.
///
/// # Examples
/// * `punnett("Aa", "Aa")` → 25% AA, 50% Aa, 25% aa
/// * `punnett("Aa", "aa")` → 50% Aa, 50% aa
pub fn punnett(parent1: &str, parent2: &str) -> Result<PunnettResult, String> {
    let p1 = parse_genotype(parent1)?;
    let p2 = parse_genotype(parent2)?;

    let gene_char = p1[0].to_ascii_lowercase();

    let mut offspring = Vec::new();
    for &a1 in &p1 {
        for &a2 in &p2 {
            // Always put dominant allele first for canonical representation
            let genotype = if a1.is_uppercase() && a2.is_lowercase() {
                format!("{}{}", a1, a2)
            } else if a1.is_lowercase() && a2.is_uppercase() {
                format!("{}{}", a2, a1)
            } else {
                format!("{}{}", a1, a2)
            };
            offspring.push(genotype);
        }
    }

    let mut genotype_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for g in &offspring {
        *genotype_counts.entry(g.clone()).or_insert(0) += 1;
    }

    let total = offspring.len() as f64;
    let recessive_char = gene_char.to_ascii_lowercase().to_string();
    let dominant_char = gene_char.to_ascii_uppercase().to_string();
    let homozygous_recessive = format!("{}{}", recessive_char, recessive_char);
    let homozygous_dominant = format!("{}{}", dominant_char, dominant_char);

    let affected_count = *genotype_counts.get(&homozygous_recessive).unwrap_or(&0) as f64;
    let unaffected_count = *genotype_counts.get(&homozygous_dominant).unwrap_or(&0) as f64;
    let carrier_count = total - affected_count - unaffected_count;

    Ok(PunnettResult {
        offspring,
        genotype_counts,
        phenotype_affected: affected_count / total,
        phenotype_carrier: carrier_count / total,
        phenotype_unaffected: unaffected_count / total,
    })
}

fn parse_genotype(genotype: &str) -> Result<Vec<char>, String> {
    let chars: Vec<char> = genotype.trim().chars().collect();
    if chars.len() != 2 {
        return Err(format!(
            "Genotype '{}' must be exactly 2 characters (e.g., 'Aa', 'AA', 'aa').",
            genotype
        ));
    }
    let base = chars[0].to_ascii_lowercase();
    if chars[1].to_ascii_lowercase() != base {
        return Err(format!(
            "Both alleles in genotype '{}' must refer to the same gene.",
            genotype
        ));
    }
    Ok(chars)
}

/// Format a Punnett square result as a human-readable string.
pub fn format_punnett(r: &PunnettResult, parent1: &str, parent2: &str) -> String {
    let mut genotypes_str = String::new();
    let mut sorted: Vec<(String, usize)> = r.genotype_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0)); // dominant first
    for (g, count) in &sorted {
        let pct = (*count as f64 / 4.0) * 100.0;
        genotypes_str.push_str(&format!("  • {} → {:.0}% ({}/4)\n", g, pct, count));
    }

    format!(
        "🧬 **Quadrado de Punnett: {} × {}**\n\
        \n\
        **Genótipos da prole:**\n\
        {}\n\
        **Fenótipos:**\n\
        • Afetados (recessivo homozigoto): {:.0}%\n\
        • Portadores (heterozigoto): {:.0}%\n\
        • Não afetados (dominante homozigoto): {:.0}%",
        parent1, parent2,
        genotypes_str,
        r.phenotype_affected * 100.0,
        r.phenotype_carrier * 100.0,
        r.phenotype_unaffected * 100.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardy_weinberg_basic() {
        // 1 in 1000 affected
        let result = hardy_weinberg(1.0, 1000.0).unwrap();
        assert!((result.q - 0.031623).abs() < 0.0001);
        assert!((result.two_pq - 0.061246).abs() < 0.0001);
    }

    #[test]
    fn test_punnett_aa_x_aa() {
        let r = punnett("Aa", "Aa").unwrap();
        assert_eq!(r.phenotype_affected, 0.25);
        assert_eq!(r.phenotype_carrier, 0.50);
        assert_eq!(r.phenotype_unaffected, 0.25);
    }

    #[test]
    fn test_punnett_aa_x_homozygous_recessive() {
        let r = punnett("Aa", "aa").unwrap();
        assert_eq!(r.phenotype_affected, 0.50);
        assert_eq!(r.phenotype_carrier, 0.50);
        assert_eq!(r.phenotype_unaffected, 0.0);
    }

    #[test]
    fn test_punnett_invalid_genotype() {
        assert!(punnett("Ab", "Aa").is_err());
    }
}
