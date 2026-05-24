export interface CentralStoreLocationPreview {
  sourcePath: string;
  targetPath: string;
  skillsToCopy: number;
  skillsToOverwrite: number;
  targetOnlySkills: number;
}

export interface CentralStoreLocationSymlinkFailure {
  table: string;
  skillId: string;
  ownerId: string;
  installedPath: string;
  error: string;
}

export interface CentralStoreLocationChangeResult {
  sourcePath: string;
  targetPath: string;
  copied: number;
  overwritten: number;
  targetOnlyImported: number;
  symlinkRebuildFailed: number;
  symlinkFailures: CentralStoreLocationSymlinkFailure[];
  completedAt: string;
}
