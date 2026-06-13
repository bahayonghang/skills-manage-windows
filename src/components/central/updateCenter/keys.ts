import type {
  DeletedPlatformCopyGroup,
  PlatformDuplicateGroup,
} from "@/types/skillUpdateInventory";

export function remoteAddedKey(repositoryId: string, sourcePath: string): string {
  return `${repositoryId} ${sourcePath}`;
}

export function duplicateGroupKey(group: PlatformDuplicateGroup): string {
  return `${group.agentId} ${group.skillId}`;
}

export function deletedPlatformCopyGroupKey(
  group: DeletedPlatformCopyGroup,
): string {
  return `${group.agentId} ${group.skillId}`;
}
