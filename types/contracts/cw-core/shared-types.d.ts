export interface Config {
    [k: string]: unknown;
    description: string;
    image_url?: (string | null);
    name: string;
}
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 *
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>
 */
export type Binary = string;
/**
 * Information about the admin of a contract.
 */
export type Admin = ({
    address: {
    addr: string
    [k: string]: unknown
    }
    } | {
    governance_contract: {
    [k: string]: unknown
    }
    } | {
    none: {
    [k: string]: unknown
    }
    });
/**
 * Information needed to instantiate a governance or voting module.
 */
export interface ModuleInstantiateInfo {
    [k: string]: unknown;
    /**
     * Admin of the instantiated contract.
     */
    admin: Admin;
    /**
     * Code ID of the contract to be instantiated.
     */
    code_id: number;
    /**
     * Label for the instantiated contract.
     */
    label: string;
    /**
     * Instantiate message to be used to create the contract.
     */
    msg: Binary;
}
