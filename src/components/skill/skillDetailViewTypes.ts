export interface DiscoverMetadata {
  name: string;
  description?: string;
  platformName: string;
  projectName: string;
  filePath: string;
  dirPath: string;
  isAlreadyCentral: boolean;
}

export type PreviewTab = "markdown" | "raw" | "explanation";
