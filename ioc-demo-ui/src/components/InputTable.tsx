import React from 'react';
import { IocBoolInput, IocFloatInput, IocStringInput, SetterFn, IocState } from '../ioc/IocWebsocketClient.tsx'

function FloatInput(props: { enabled: boolean, input: IocFloatInput, setter: (x: number) => void }) {
    return <>
        <input 
            disabled={!props.enabled}
            type="range" 
            min={props.input.min}
            max={props.input.max} 
            step={props.input.step} 
            value={props.input.value} 
            onChange={evt => props.setter(parseFloat(evt.target.value) + 0.0)} 
        />&nbsp;{props.input.value}
    </>;
}

function BoolInput(props: { enabled: boolean, input: IocBoolInput, setter: (x: boolean) => void }) {
    return <>
        <input 
            disabled={!props.enabled}
            type="checkbox"
            checked={ props.input.value }
            onChange={ () => props.setter( !props.input.value ) }
        />
        {props.input.value}
    </>
}

function StringInput(props: { enabled: boolean, input: IocStringInput, setter: (x: string) => void }) {
    return <>
        <input 
            disabled={!props.enabled}
            type="text"
            value={ props.input.value }
            onChange={ evt => props.setter( evt.target.value) }
        />
    </>
}

export default function InputTable(props: { ioc: IocState , setter: SetterFn } ) {

    let rows: React.ReactElement[] = [];
    
    Object.keys(props.ioc.inputs).forEach(k => {
        let input = props.ioc.inputs[k];

        let element = <>unsupported input type</>;

        if("Float" in input) {
            element = <FloatInput enabled={props.ioc.connected} input={input.Float} setter={n => props.setter(k, n)} />;
        } else if("Bool" in input) {
            element = <BoolInput enabled={props.ioc.connected} input={input.Bool} setter={b => props.setter(k, b)}/>;
        } else if("String" in input) {
            element = <StringInput enabled={props.ioc.connected} input={input.String} setter={s => props.setter(k, s)} />;
        }

        rows.push(
            <tr key={k} >
                <td className="tableKey">
                    {k}
                </td>
                <td className="tableValue">
                    {element}
                </td>
            </tr>
        );
    });

    return <>
        <h3>Inputs</h3>
        <table>
            <tbody>
            {rows}
            </tbody>
        </table>
    </>
}