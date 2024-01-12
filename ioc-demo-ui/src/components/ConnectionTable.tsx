import { IocState } from '../ioc/IocWebsocketClient';
import ReactTimeAgo from 'react-time-ago';

import en from 'javascript-time-ago/locale/en.json'

import TimeAgo from 'javascript-time-ago'
TimeAgo.addDefaultLocale(en)

export default function ConnectionTable(props: { ioc: IocState }) {

    return <>
        <h3>Websocket</h3>
        <table>
            <tbody>
                <tr>
                    <td className="tableKey">
                        status
                    </td>
                    <td className="tableValue">
                        {props.ioc.status}
                    </td>
                </tr>
                <tr>
                    <td className="tableKey">
                        last timestamp
                    </td>
                    <td className="tableValue">
                        {props.ioc.time?.seconds}
                    </td>
                </tr>
                <tr>
                    <td className="tableKey">
                        uptime
                    </td>
                    <td className="tableValue">
                        {props.ioc.upSince ? <ReactTimeAgo date={props.ioc.upSince} locale="en-US" timeStyle="mini"/> : <>-</>}
                    </td>
                </tr>
            </tbody>
        </table>
    </>
}