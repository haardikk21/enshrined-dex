'use client'

import { type Log } from 'viem'
import { decodeLog, formatArg, getEventDescription } from '@/lib/logDecoder'
import { AddressDisplay } from './AddressDisplay'

interface LogDisplayProps {
  log: Log
  index: number
}

export function LogDisplay({ log, index }: LogDisplayProps) {
  const decoded = decodeLog(log)

  return (
    <div className="border border-white/10 rounded-xl p-4 bg-white/5">
      <div className="flex items-center justify-between mb-3">
        <div className="text-sm font-medium text-white">Log #{index}</div>
        {decoded.success && (
          <span className="inline-flex items-center px-3 py-1 rounded-lg text-xs font-medium bg-[#0052ff]/20 text-[#0052ff] border border-[#0052ff]/30">
            {decoded.eventName}
          </span>
        )}
      </div>

      <dl className="space-y-3">
        <div>
          <dt className="text-xs font-medium text-white/50 mb-1">Address</dt>
          <dd className="text-xs text-white/90">
            <AddressDisplay address={log.address} />
          </dd>
        </div>

        {decoded.success ? (
          <>
            {/* Event Description */}
            <div>
              <dt className="text-xs font-medium text-white/50 mb-1">Event</dt>
              <dd className="text-xs text-white/90">
                {getEventDescription(decoded.eventName, decoded.args, log.address)}
              </dd>
            </div>

            {/* Decoded Arguments */}
            <div>
              <dt className="text-xs font-medium text-white/50 mb-2">Decoded Parameters</dt>
              <dd className="space-y-2">
                {Object.entries(decoded.args).map(([key, value]) => {
                  // Skip numeric keys (which are duplicates of named keys)
                  if (!isNaN(Number(key))) return null

                  // Check if value is an address
                  const isAddress = typeof value === 'string' && value.startsWith('0x') && value.length === 42

                  return (
                    <div key={key} className="bg-black/30 p-2 rounded">
                      <div className="text-xs font-medium text-[#0052ff] mb-1">{key}</div>
                      <div className="text-xs text-white/90">
                        {isAddress ? (
                          <AddressDisplay address={value as string} />
                        ) : (
                          <span className="font-mono break-all">{formatArg(key, value)}</span>
                        )}
                      </div>
                    </div>
                  )
                })}
              </dd>
            </div>
          </>
        ) : (
          <>
            {/* Failed to decode - show raw data */}
            <div>
              <dt className="text-xs font-medium text-white/50 mb-1">
                <span className="text-yellow-500">⚠️ Unable to decode</span>
              </dt>
              <dd className="text-xs text-white/50 italic">
                {decoded.error}
              </dd>
            </div>

            {log.topics && log.topics.length > 0 && (
              <div>
                <dt className="text-xs font-medium text-white/50 mb-1">Topics</dt>
                <dd className="text-xs text-white/90 font-mono space-y-1">
                  {log.topics.map((topic, i) => (
                    <div key={i} className="break-all">
                      [{i}] {topic}
                    </div>
                  ))}
                </dd>
              </div>
            )}

            {log.data && log.data !== '0x' && (
              <div>
                <dt className="text-xs font-medium text-white/50 mb-1">Data</dt>
                <dd className="text-xs text-white/90 font-mono break-all bg-black/30 p-2 rounded">
                  {log.data}
                </dd>
              </div>
            )}
          </>
        )}
      </dl>
    </div>
  )
}
