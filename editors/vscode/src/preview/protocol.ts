export interface PreviewFile {
    uri: string;
    path: string;
    before: string;
    result: string;
}

export interface PreviewError {
    path: string;
    message: string;
}

export interface OperationResult {
    files: PreviewFile[];
    errors: PreviewError[];
}

export interface PreviewResult extends OperationResult {
    matched: number;
}
