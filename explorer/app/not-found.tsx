import Link from 'next/link'

export default function NotFound() {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-8xl font-bold text-white mb-6 tracking-tight">404</h1>
        <h2 className="text-2xl font-semibold text-white mb-3">
          Resource Not Found
        </h2>
        <p className="text-white/60 mb-10 text-lg">
          The block or transaction you&apos;re looking for doesn&apos;t exist.
        </p>
        <Link
          href="/"
          className="inline-flex items-center px-6 py-3 text-sm font-medium rounded-xl text-white bg-[#0052ff] hover:bg-[#0052ff]/90 transition-colors"
        >
          Back to Explorer
        </Link>
      </div>
    </div>
  )
}
