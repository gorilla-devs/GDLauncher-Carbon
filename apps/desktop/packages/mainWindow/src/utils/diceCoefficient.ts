interface ResultItem {
  id: string
  downloadsCount?: number
  title?: string
  description?: string
  [key: string]: any // Allow for additional properties
}

interface ScoredResultItem extends ResultItem {
  similarityScore: number
}

function diceCoefficient(first: string, second: string): number {
  // Handle edge cases
  if (first.length < 2 || second.length < 2) {
    return 0
  }

  // Special case for exact matches
  if (first === second) {
    return 1
  }

  // Map to count bigrams in the first string
  const bigramCounts = new Map<string, number>()

  // Count bigrams in the first string
  for (let i = 0; i < first.length - 1; i++) {
    const bigram = first.substring(i, i + 2)
    const count = bigramCounts.get(bigram) || 0
    bigramCounts.set(bigram, count + 1)
  }

  // Count matches in the second string
  let matches = 0
  for (let i = 0; i < second.length - 1; i++) {
    const bigram = second.substring(i, i + 2)
    const count = bigramCounts.get(bigram) || 0
    if (count > 0) {
      matches++
      bigramCounts.set(bigram, count - 1)
    }
  }

  // Calculate and return Dice coefficient
  return (2 * matches) / (first.length + second.length - 2)
}

function getSimilarResults(
  results: ResultItem[],
  query: string,
  limit = 3,
  titleWeight = 0.7,
  descriptionWeight = 0.3
): string[] {
  // Check for valid input
  if (!results || results.length === 0 || !query) {
    return []
  }

  // Normalize query
  const normalizedQuery = query.toLowerCase().trim()

  // Normalize weights to ensure they sum to 1
  const totalWeight = titleWeight + descriptionWeight
  const normalizedTitleWeight = titleWeight / totalWeight
  const normalizedDescWeight = descriptionWeight / totalWeight

  // Calculate similarity scores
  const scoredResults: ScoredResultItem[] = results.map((result) => {
    const title = (result.title || "").toLowerCase().trim()
    const description = (result.description || "").toLowerCase().trim()

    // Calculate Dice coefficient for title and description
    const titleSimilarity = diceCoefficient(normalizedQuery, title)
    const descriptionSimilarity = diceCoefficient(normalizedQuery, description)

    // Apply exact match bonuses
    const titleBonus = title.includes(normalizedQuery) ? 0.2 : 0
    const descBonus = description.includes(normalizedQuery) ? 0.1 : 0

    // Calculate final weighted score
    const titleScore = Math.min(1, titleSimilarity + titleBonus)
    const descScore = Math.min(1, descriptionSimilarity + descBonus)
    const totalScore =
      titleScore * normalizedTitleWeight + descScore * normalizedDescWeight

    return {
      ...result,
      similarityScore: totalScore
    }
  })

  // Sort by score in descending order
  scoredResults.sort((a, b) => b.similarityScore - a.similarityScore)

  // Return top N result ids
  return scoredResults.slice(0, limit).map((result) => result.id)
}

function getEnhancedSimilarResults(
  results: ResultItem[],
  query: string,
  limit = 10,
  titleWeight = 0.7,
  descriptionWeight = 0.3
): string[] {
  // Check for valid input
  if (!results || results.length === 0 || !query) {
    return []
  }

  // Normalize query and split into words
  const normalizedQuery = query.toLowerCase().trim()
  const queryWords = normalizedQuery
    .split(/\s+/)
    .filter((word) => word.length > 1)

  // Normalize weights
  const totalWeight = titleWeight + descriptionWeight
  const normalizedTitleWeight = titleWeight / totalWeight
  const normalizedDescWeight = descriptionWeight / totalWeight

  // Calculate similarity scores with enhanced matching
  const scoredResults: ScoredResultItem[] = results.map((result) => {
    const title = (result.title || "").toLowerCase().trim()
    const description = (result.description || "").toLowerCase().trim()

    // Basic similarity with Dice coefficient
    let titleSimilarity = diceCoefficient(normalizedQuery, title)
    let descSimilarity = diceCoefficient(normalizedQuery, description)

    // Word-level matching for better precision
    if (queryWords.length > 0) {
      const titleWords = title.split(/\s+/).filter((word) => word.length > 1)
      const descWords = description
        .split(/\s+/)
        .filter((word) => word.length > 1)

      // Count exact word matches
      const titleWordMatches = queryWords.filter((word) =>
        titleWords.includes(word)
      ).length
      const descWordMatches = queryWords.filter((word) =>
        descWords.includes(word)
      ).length

      // Calculate word match ratios
      const titleWordRatio = titleWordMatches / queryWords.length
      const descWordRatio = descWordMatches / queryWords.length

      // Add word match bonuses (max 0.3 for title, 0.2 for description)
      titleSimilarity = Math.min(1, titleSimilarity + titleWordRatio * 0.3)
      descSimilarity = Math.min(1, descSimilarity + descWordRatio * 0.2)
    }

    // Additional bonuses for exact phrase matches
    if (title === normalizedQuery) titleSimilarity = 1
    else if (title.includes(normalizedQuery))
      titleSimilarity = Math.min(1, titleSimilarity + 0.2)

    if (description === normalizedQuery) descSimilarity = 1
    else if (description.includes(normalizedQuery))
      descSimilarity = Math.min(1, descSimilarity + 0.1)

    // Calculate final weighted score
    const totalScore =
      titleSimilarity * normalizedTitleWeight +
      descSimilarity * normalizedDescWeight

    // Add download count bonus (normalized to avoid overwhelming the similarity score)
    let downloadBonus = 0
    if (result.downloadsCount) {
      // Log scale to prevent extremely popular mods from dominating
      downloadBonus = Math.min(0.2, Math.log10(result.downloadsCount + 1) / 10)
    }

    return {
      ...result,
      similarityScore: totalScore + downloadBonus
    }
  })

  // Sort by score in descending order
  scoredResults.sort((a, b) => b.similarityScore - a.similarityScore)

  // Return top N result ids
  return scoredResults.slice(0, limit).map((result) => result.id)
}

export { getSimilarResults, getEnhancedSimilarResults, diceCoefficient }
