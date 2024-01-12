import React from 'react';
import { IocState } from '../ioc/IocWebsocketClient.tsx'

export default function OutputTable(props: { ioc: IocState }) {

    let rows: React.ReactElement[] = [];

    Object.keys(props.ioc.outputs).forEach(k => {
        let output = props.ioc.outputs[k];
        let element = <>unsupported output type</>;
        if("Float" in output) {
            element = <>{output.Float.value.toString()}</>;
        } else if("Bool" in output) {
            if(output.Bool.value) {
                element = <>✅&nbsp;{output.Bool.value.toString()}</>;
            } else {
                element = <>❌&nbsp;{output.Bool.value.toString()}</>;
            }
        } else if("String" in output) {
            element = <>{output.String.value}</>;
        }

        rows.push(
            <tr key={k}>
                <td className="tableKey">
                    {k}
                </td>
                <td className="tableValue">
                    {element}
                </td>
            </tr>
        )
    });

    return <>
        <h3>Outputs</h3>
        <table>
            <tbody>
            {rows}
            </tbody>
        </table>
    </>
}