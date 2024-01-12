import { useCallback, useState, useEffect, useReducer, useMemo } from 'react';
import { IocState } from '../ioc/IocWebsocketClient';
import { useMeasure } from '@uidotdev/usehooks';
import * as d3 from 'd3';


interface ChartFloatValueState {
    color: string,
    points: {t: number, y: number}[];
}

type ChartValueState = { Float: ChartFloatValueState };

interface ChartState {
    timeOffset: number | null,
    inputs: { [key: string]: ChartValueState};
    outputs: { [key: string]: ChartValueState};
}

function pad(n: number) {
    let s = n.toString(16);
    while(s.length < 2) {
        s = "0" + s;
    }
    return s;
}

function random_color(center: number[], cube: number[]) {

    let r = Math.max(Math.min( Math.round(((Math.random()-0.5)*cube[0]+center[0])*255), 255), 0);
    let g = Math.max(Math.min( Math.round(((Math.random()-0.5)*cube[1]+center[1])*255), 255), 0);
    let b = Math.max(Math.min( Math.round(((Math.random()-0.5)*cube[2]+center[2])*255), 255), 0);

    let color = "#" + pad(r) + pad(g) + pad(b);
    
    return color;
}

function chartStateReducer(chartState: ChartState, action: IocState): ChartState {

    const history = 10; //seconds

    let newChartState: ChartState = JSON.parse(JSON.stringify(chartState));

    if(action.time) {
        let serverTime = action.time.seconds;
        let clientTime = new Date().getTime() / 1000;

        if(newChartState.timeOffset === null) {
            newChartState.timeOffset = clientTime - serverTime;
        }

        for(const k in action.inputs) if(k.length > 1) {
            let input = action.inputs[k];
            let chartValState: ChartValueState | undefined = newChartState.inputs[k];
            if('Float' in input) {
                let floatInput = input.Float;
                if(chartValState) {
                    if('Float' in chartValState) {
                        let floatState = chartValState.Float;
                        if(floatState.points[0].t !== serverTime && floatState.points[0].y !== floatInput.value) {
                            floatState.points.unshift({t: serverTime, y: floatState.points[0].y});
                            floatState.points.unshift({t: serverTime, y: floatInput.value});
                        }
                        while(floatState.points.length > 2 && floatState.points[floatState.points.length - 2].t < serverTime - history) {
                            floatState.points.pop();
                        }
                    } else {
                        console.error("bad!", chartValState);
                    }
                } else {
                    chartValState = {
                        Float: {
                            color: random_color([0.2, 0.2, 0.6], [0.5, 0.5, 0.5]),
                            points: [
                                { t: serverTime, y: floatInput.value }
                            ]
                        }
                    };
                }
            } else {
                console.error("bad!");
            }
            newChartState.inputs[k] = chartValState;
        }

        for(const k in action.outputs) {
            let output = action.outputs[k];
            let chartValState: ChartValueState | undefined = newChartState.outputs[k];
            if('Float' in output) {
                let floatOutput = output.Float;
                if(chartValState) {
                    if('Float' in chartValState) {
                        let floatState = chartValState.Float;

                        if(floatState.points[0].t != serverTime && floatState.points[0].y != floatOutput.value) {
                            floatState.points.unshift({t: serverTime, y: floatState.points[0].y});
                            floatState.points.unshift({t: serverTime, y: floatOutput.value});
                        }
                        while(floatState.points.length > 2 && floatState.points[floatState.points.length - 2].t < serverTime - history) {
                            floatState.points.pop();
                        }
                    }
                } else {
                    chartValState = {
                        Float: {
                            color: random_color([0.6, 0.2, 0.2], [0.5, 0.5, 0.5]),
                            points: [
                                {t: serverTime, y: floatOutput.value}
                            ]
                        }
                    }
                }

            }

            newChartState.outputs[k] = chartValState;
        }
    }


    return newChartState;
}

function ChartLine(props: {t0: number, values: ChartFloatValueState, lineBuilder: d3.Line<[number, number]>}) {

    let pts: [number, number][] = [];

    let head = props.values.points[0];
    if(head) {
        pts.push([0.0, head.y]);
    }

    props.values.points.forEach( pt => {
        pts.push([props.t0-pt.t, pt.y]);
    });

    let path = props.lineBuilder(pts);

    return <>
        <path d={path ? path : undefined} stroke={props.values.color} fill="none" strokeWidth={2} />
    </>
}

export default function Chart(props: {ioc: IocState}) {

    const ioc = props.ioc;
    const [chartState, updateChartState] = useReducer(chartStateReducer, {timeOffset: null, inputs: {}, outputs: {}});

    const [tStart, setTStart] = useState(new Date().getTime() / 1000);
    const [tOffs, setTOffs] = useState(0);
    const [animationFrame, setAnimationFrame] = useState(0); 

    const animationCallback = useCallback(() => {
        if(ioc.time) {
            let t = new Date().getTime() / 1000;
            let too = t - tStart;
            setTOffs(too);
        }
        window.requestAnimationFrame(animationCallback);
    }, [ioc, tStart]);

    useEffect(() => {
        setTOffs(0);
        setTStart(new Date().getTime() / 1000);
        updateChartState(ioc);
        window.cancelAnimationFrame(animationFrame);
        setAnimationFrame(window.requestAnimationFrame(animationCallback));
    }, [ioc, updateChartState]);

    //dimensions of svg element in pixels
    const [ref, { width, height }] = useMeasure();
    const final_width: number = width === null ? 300 : width;
    const final_height: number = height === null ? 200 : height;

    //margins in pixels between svg element and chart area
    //must leave room for axes on bottom and left
    const margin = {left: 50, right: 25, bottom: 30, top: 25}; 
    
    //data domain 
    const secondsHistory = 10;
    const valueMin = -1-1/3;
    const valueMax = 1+1/3;

    //scales from data domain to pixels
    const tScale = d3.scaleLinear().domain([0, secondsHistory]).range([0, final_width-margin.left-margin.right]);
    const yScale = d3.scaleLinear().domain([valueMax, valueMin]).range([0, final_height-margin.top-margin.bottom]);

    const t0 = (ioc.time?.seconds ? ioc.time.seconds : 0.0) + tOffs;
    const lineBuilder = d3.line().x( d => tScale(d[0]) ).y( d => yScale(d[1]) );
    let paths = [];
    
    for(const k in chartState.inputs) {
        let valueState = chartState.inputs[k];
        if('Float' in valueState) {
            let floatValueState = valueState.Float;
            paths.push(<ChartLine t0={t0} key={"fin-"+k} values={floatValueState} lineBuilder={lineBuilder} />);
        }
    }

    for(const k in chartState.outputs) {
        let valueState = chartState.outputs[k];
        if('Float' in valueState) {
            let floatValueState = valueState.Float;
            paths.push(<ChartLine t0={t0} key={"fout-"+k} values={floatValueState} lineBuilder={lineBuilder} />)
        }
    }

    return <>
        <h3>Chart</h3>
        <svg className="chart" ref={ref}>
            <g
                width={final_width}
                height={final_height}
                transform={`translate(${[margin.left, 0].join(",")})`}
                overflow={"visible"}
            >
                <g transform={`translate(${[0, final_height-margin.bottom].join(",")})`} >
                    <TAxis scale={tScale} pixelsPerTick={100} />
                </g>
                <g transform={`translate(${[0, margin.top].join(",")})`} >
                    <YAxis scale={yScale} pixelsPerTick={25} />
                    {paths}
                </g>
                



            </g>
        </svg>
        <div>
            <pre>
                {/* {JSON.stringify(chartState, null, 2)} */}
            </pre>
        </div>
    </>
}


function TAxis(props: { scale: d3.ScaleLinear<number, number>, pixelsPerTick: number }) {

    const range = props.scale.range();

    const ticks = useMemo(() => {
      const width = range[1] - range[0];
      const numberOfTicksTarget = Math.floor(width / props.pixelsPerTick);
  
      return props.scale.ticks(numberOfTicksTarget).map((value: number) => ({
        value,
        xOffset: props.scale(value),
      }));
    }, [props]);
  
    return (
      <>
        {/* Main horizontal line */}
        <path
          d={["M", range[0], 0, "L", range[1], 0].join(" ")}
          fill="none"
          stroke="currentColor"
        />
  
        {/* Ticks and labels */}
        {ticks.map((o: { value: number, xOffset: number }) => (
          <g key={o.value} transform={`translate(${o.xOffset}, 0)`}>
            <line y2={5} stroke="currentColor" />
            <text
              key={o.value}
              style={{
                fontSize: "10px",
                textAnchor: "middle",
                transform: "translateY(20px)",
              }}
            >
              {o.value}
            </text>
          </g>
        ))}
      </>
    );
}

function YAxis(props: {scale: d3.ScaleLinear<number, number>, pixelsPerTick: number}) {
    const range = props.scale.range();

    const ticks = useMemo(() => {
        const height = range[1]-range[0];
        const numberOfTicksTarget = Math.floor(height / props.pixelsPerTick);

        return props.scale.ticks(numberOfTicksTarget).map((value) => ({
            value, yOffset: props.scale(value),
        }));
    }, [props]);

    return (
        <>
            <path 
                d={["M", 0, range[0], "L", 0, range[1]].join(" ")}
                fill="none"
                stroke="currentColor"
            />

            {ticks.map(({ value, yOffset }) => (
                <g key={value} transform={`translate(0, ${yOffset})`}>
                <line x2={-5} stroke="currentColor" />
                <text
                    key={value}
                    style={{
                    fontSize: "10px",
                    textAnchor: "middle",
                    transform: "translateX(-20px)",
                    }}
                >
                    {value}
                </text>
                </g>
            ))}

        </>
    )
}