export class DocumentPersistenceError extends Error {
  constructor(message = 'native Documents persistence is unavailable') {
    super(message);
    this.name = 'DocumentPersistenceError';
  }
}

export class DocumentCheckpointChangedError extends DocumentPersistenceError {
  constructor() {
    super('native Documents checkpoint changed');
    this.name = 'DocumentCheckpointChangedError';
  }
}
