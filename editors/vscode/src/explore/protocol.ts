export interface PlainPosition {
    line: number;
    character: number;
}

export interface PlainRange {
    start: PlainPosition;
    end: PlainPosition;
}

export interface PlainLocation {
    uri: string;
    range: PlainRange;
}

export interface ExploredFile {
    uri: string;
    path: string;
    language: string;
    usual: string | null;
}

export interface FilesResult {
    files: ExploredFile[];
    without: number;
}

export interface ExploredFrom extends FilesResult {
    patch?: string | null;
    location?: string | null;
}

export interface BlockName {
    kind: string;
    field: string | null;
}

export interface BlockPath {
    location: string;
    places: number;
    ranges: PlainRange[];
    number: number | null;
}

export interface BlockUse {
    uri: string;
    patch: string;
    action: string;
    field: string;
    enabled: boolean;
    range: PlainRange;
    whole: boolean;
}

export interface Block extends BlockName {
    broken: boolean;
    range: PlainRange;
    paths: BlockPath[];
    query: string | null;
    queryLanguage?: { name: string; documentation: string } | null;
    parents: (BlockName & { range: PlainRange })[];
    child: PlainRange | null;
    used: BlockUse[];
}

export interface Place extends ExploredFile {
    range: PlainRange;
    number: number;
    places: number;
}

export interface HintPart {
    node: number;
    label: string;
    location: string;
    range: PlainRange;
}

export interface BlockHint {
    position: PlainPosition;
    parts: HintPart[];
}
