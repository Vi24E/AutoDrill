export const CUSTOM_SAMPLING_PROFILE = Object.freeze({
  name: 'operation_vector_information',
  version: '1',
});

const RIDGE_INFORMATION_PRIOR = 1;
const MIN_SCALE = 1e-9;
const MIN_CHOLESKY_DIAGONAL = 1e-12;

function rawFeatureVector(diagnostic) {
  if (Array.isArray(diagnostic?.operation_vector)) return diagnostic.operation_vector.map(Number);
  return [Number(diagnostic?.effort ?? 0)];
}

function rmsScales(rows, dimension) {
  const totals = Array(dimension).fill(0);
  for (const row of rows) {
    const vector = rawFeatureVector(row);
    for (let index = 0; index < dimension; index += 1) {
      const value = Number(vector[index] ?? 0);
      totals[index] += value * value;
    }
  }
  const divisor = Math.max(1, rows.length);
  return totals.map((total) => Math.max(Math.sqrt(total / divisor), MIN_SCALE));
}

function standardizedFeatureVector(row, scales, dimension) {
  const raw = rawFeatureVector(row);
  return [1, ...Array.from({ length: dimension }, (_, index) => Number(raw[index] ?? 0) / scales[index])];
}

function ridgeInformationMatrix(vectors) {
  const dimension = vectors[0].length;
  const matrix = Array.from({ length: dimension }, (_, row) => (
    Array.from({ length: dimension }, (_, column) => (row === column ? RIDGE_INFORMATION_PRIOR : 0))
  ));
  for (const vector of vectors) {
    for (let row = 0; row < dimension; row += 1) {
      for (let column = 0; column <= row; column += 1) {
        const value = vector[row] * vector[column];
        matrix[row][column] += value;
        if (row !== column) matrix[column][row] += value;
      }
    }
  }
  return matrix;
}

function cholesky(matrix) {
  const dimension = matrix.length;
  const lower = Array.from({ length: dimension }, () => Array(dimension).fill(0));
  for (let row = 0; row < dimension; row += 1) {
    for (let column = 0; column <= row; column += 1) {
      let value = matrix[row][column];
      for (let inner = 0; inner < column; inner += 1) value -= lower[row][inner] * lower[column][inner];
      if (row === column) {
        lower[row][column] = Math.sqrt(Math.max(value, MIN_CHOLESKY_DIAGONAL));
      } else {
        lower[row][column] = value / lower[column][column];
      }
    }
  }
  return lower;
}

function ridgeLeverage(vector, lower) {
  const dimension = vector.length;
  const forward = Array(dimension).fill(0);
  for (let row = 0; row < dimension; row += 1) {
    let value = vector[row];
    for (let column = 0; column < row; column += 1) value -= lower[row][column] * forward[column];
    forward[row] = value / lower[row][row];
  }
  const solved = Array(dimension).fill(0);
  for (let row = dimension - 1; row >= 0; row -= 1) {
    let value = forward[row];
    for (let column = row + 1; column < dimension; column += 1) value -= lower[column][row] * solved[column];
    solved[row] = value / lower[row][row];
  }
  return vector.reduce((total, value, index) => total + value * solved[index], 0);
}

/**
 * Rank QA candidates by ridge leverage in the current effort-operation feature
 * space. This is an experimental-design score, not a problem difficulty score:
 * high values identify candidates that add information in directions weakly
 * covered by completed observations. The same design supports estimating both
 * User-rated axes without treating either rating as ground truth.
 */
export function scoreInformationCandidates({ observed = [], candidates = [] } = {}) {
  if (!candidates.length) return [];
  const rawVectors = [...observed, ...candidates].map(rawFeatureVector);
  const operationDimension = Math.max(1, ...rawVectors.map((vector) => vector.length));
  const scales = rmsScales([...observed, ...candidates], operationDimension);
  const observedVectors = observed.map((row) => standardizedFeatureVector(row, scales, operationDimension));
  const featureDimension = operationDimension + 1;
  const priorVector = Array(featureDimension).fill(0);
  const information = ridgeInformationMatrix(observedVectors.length ? observedVectors : [priorVector]);
  const lower = cholesky(information);
  return candidates.map((candidate) => {
    const vector = standardizedFeatureVector(candidate, scales, operationDimension);
    return { ...candidate, information_score: ridgeLeverage(vector, lower) };
  });
}
