import React from 'react';
import { GridRows, GridColumns } from '@visx/grid';
import { scaleLinear } from '@visx/scale';
import { AxisBottom, AxisLeft } from '@visx/axis';
import { Group } from '@visx/group';

interface GridChartProps {
    width: number;
    height: number;
    maxX: number;
    maxY: number;
}

const margin = { top: 20, right: 20, bottom: 40, left: 40 };

const GridChart: React.FC<GridChartProps> = ({ width, height, maxX, maxY }) => {
    // Calculate chart dimensions
    const xMax = width - margin.left - margin.right;
    const yMax = height - margin.top - margin.bottom;

    // Create scales for x and y axes, supporting negative coordinates
    const xScale = scaleLinear({
        domain: [-maxX, maxX], // Support negative x values
        range: [0, xMax],
    });

    const yScale = scaleLinear({
        domain: [-maxY, maxY], // Support negative y values
        range: [yMax, 0], // Flip the y scale so negative values are below the origin
    });

    // Calculate the center for the origin (x=0, y=0)
    const xCenter = xScale(0);
    const yCenter = yScale(0);

    return (
        <svg width={width} height={height}>
            <Group left={margin.left} top={margin.top}>
                {/* Draw the grid */}
                <GridRows
                    scale={yScale}
                    width={xMax}
                    stroke="lightgray"
                />
                <GridColumns
                    scale={xScale}
                    height={yMax}
                    stroke="lightgray"
                />

                {/* Draw the x and y axes */}
                <AxisBottom
                    top={yCenter} // Position x-axis at the y-center (origin)
                    scale={xScale}
                    stroke="black"
                    tickStroke="black"
                    tickLabelProps={() => ({
                        fill: 'black',
                        fontSize: 11,
                        textAnchor: 'middle',
                    })}
                />
                <AxisLeft
                    left={xCenter} // Position y-axis at the x-center (origin)
                    scale={yScale}
                    stroke="black"
                    tickStroke="black"
                    tickLabelProps={() => ({
                        fill: 'black',
                        fontSize: 11,
                        textAnchor: 'end',
                        dx: '-0.25em',
                    })}
                />
            </Group>
        </svg>
    );
};

export default GridChart;