"use client";

import { getNameTag, formatAddress } from "@/lib/nametags";

interface AddressDisplayProps {
  address: string;
  className?: string;
  showFull?: boolean;
}

export function AddressDisplay({
  address,
  className = "",
  showFull = false,
}: AddressDisplayProps) {
  const nameTag = getNameTag(address);

  if (nameTag) {
    return (
      <span
        className={`inline-flex items-center gap-1 ${className}`}
        title={address}
      >
        <span className="px-2 py-0.5 bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200 rounded-md text-sm font-medium">
          {nameTag}
        </span>
        {showFull && (
          <span className="text-xs text-gray-500 dark:text-gray-400 font-mono">
            ({formatAddress(address)})
          </span>
        )}
      </span>
    );
  }

  return (
    <span className={`font-mono ${className}`} title={address}>
      {formatAddress(address)}
    </span>
  );
}
