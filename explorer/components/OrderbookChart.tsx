"use client";

interface Order {
  price: number;
  amount: number;
  total: number;
}

interface OrderbookChartProps {
  buyOrders: Order[];
  sellOrders: Order[];
  token0Name: string;
  token1Name: string;
}

export function OrderbookChart({
  buyOrders,
  sellOrders,
  token0Name,
  token1Name,
}: OrderbookChartProps) {
  // Find the price range
  const allPrices = [
    ...buyOrders.map((o) => o.price),
    ...sellOrders.map((o) => o.price),
  ];
  const minPrice = Math.min(...allPrices);
  const maxPrice = Math.max(...allPrices);
  const priceRange = maxPrice - minPrice;

  // Recalculate cumulative totals for proper depth chart
  // Buy orders: cumulative from highest price (best bid) down
  const buyOrdersWithDepth = [...buyOrders].reverse().map((order, i, arr) => ({
    ...order,
    depth: arr.slice(0, i + 1).reduce((sum, o) => sum + o.amount, 0),
  }));

  // Sell orders: cumulative from lowest price (best ask) up
  const sellOrdersWithDepth = sellOrders.map((order, i, arr) => ({
    ...order,
    depth: arr.slice(0, i + 1).reduce((sum, o) => sum + o.amount, 0),
  }));

  // Find max depth for scaling
  const maxDepth = Math.max(
    ...buyOrdersWithDepth.map((o) => o.depth),
    ...sellOrdersWithDepth.map((o) => o.depth),
  );

  // Chart dimensions
  const chartHeight = 400;
  const chartPadding = 40;

  return (
    <div className="w-full">
      {/* Stats */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="bg-black/30 rounded-lg p-4">
          <div className="text-xs text-white/50 mb-1">Best Bid</div>
          <div className="text-lg font-semibold text-green-400 font-mono">
            {buyOrdersWithDepth[0]?.price.toFixed(2) || "—"}
          </div>
        </div>
        <div className="bg-black/30 rounded-lg p-4">
          <div className="text-xs text-white/50 mb-1">Total Depth</div>
          <div className="text-lg font-semibold text-white font-mono">
            {maxDepth.toFixed(2)} {token0Name}
          </div>
        </div>
        <div className="bg-black/30 rounded-lg p-4">
          <div className="text-xs text-white/50 mb-1">Spread</div>
          <div className="text-lg font-semibold text-white font-mono">
            {(sellOrdersWithDepth[0]?.price - buyOrdersWithDepth[0]?.price).toFixed(2)} {token1Name}
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="relative bg-black/30 rounded-lg p-4" style={{ height: chartHeight }}>
        {/* Y-axis labels */}
        <div className="absolute left-0 top-0 bottom-0 flex flex-col justify-between py-8 pr-2 text-xs text-white/50">
          <div>{maxDepth.toFixed(1)}</div>
          <div>{(maxDepth / 2).toFixed(1)}</div>
          <div>0</div>
        </div>

        {/* Chart area */}
        <div className="relative ml-12 h-full">
          <svg className="w-full h-full" viewBox={`0 0 100 ${chartHeight}`} preserveAspectRatio="none">
            {/* Buy orders - green area */}
            {buyOrdersWithDepth.length > 0 && (
              <polygon
                points={buyOrdersWithDepth
                  .map((order) => {
                    const x = ((order.price - minPrice) / priceRange) * 100;
                    const y = chartHeight - (order.depth / maxDepth) * (chartHeight - chartPadding * 2) - chartPadding;
                    return `${x},${y}`;
                  })
                  .concat([
                    `${((buyOrdersWithDepth[buyOrdersWithDepth.length - 1].price - minPrice) / priceRange) * 100},${chartHeight - chartPadding}`,
                    `${((buyOrdersWithDepth[0].price - minPrice) / priceRange) * 100},${chartHeight - chartPadding}`,
                  ])
                  .join(" ")}
                fill="rgba(34, 197, 94, 0.2)"
                stroke="rgb(34, 197, 94)"
                strokeWidth="0.5"
              />
            )}

            {/* Sell orders - red area */}
            {sellOrdersWithDepth.length > 0 && (
              <polygon
                points={sellOrdersWithDepth
                  .map((order) => {
                    const x = ((order.price - minPrice) / priceRange) * 100;
                    const y = chartHeight - (order.depth / maxDepth) * (chartHeight - chartPadding * 2) - chartPadding;
                    return `${x},${y}`;
                  })
                  .concat([
                    `${((sellOrdersWithDepth[sellOrdersWithDepth.length - 1].price - minPrice) / priceRange) * 100},${chartHeight - chartPadding}`,
                    `${((sellOrdersWithDepth[0].price - minPrice) / priceRange) * 100},${chartHeight - chartPadding}`,
                  ])
                  .join(" ")}
                fill="rgba(239, 68, 68, 0.2)"
                stroke="rgb(239, 68, 68)"
                strokeWidth="0.5"
              />
            )}
          </svg>

          {/* X-axis labels */}
          <div className="flex justify-between mt-2 text-xs text-white/50">
            <div>{minPrice.toFixed(0)}</div>
            <div>{((minPrice + maxPrice) / 2).toFixed(0)}</div>
            <div>{maxPrice.toFixed(0)}</div>
          </div>
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center justify-center gap-6 mt-4 text-sm">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-green-500 rounded"></div>
          <span className="text-white/70">Buy Orders</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-red-500 rounded"></div>
          <span className="text-white/70">Sell Orders</span>
        </div>
      </div>
    </div>
  );
}
