# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0

from odxtools.diaglayers.diaglayerraw import DiagLayerRaw
from odxtools.diagservice import DiagService
from odxtools.nameditemlist import NamedItemList
from odxtools.parameters.valueparameter import ValueParameter
from odxtools.request import Request
from odxtools.response import Response, ResponseType

from helper import (
    derived_id,
    functional_class_ref,
    matching_request_parameter,
    matching_request_parameter_subfunction,
    ref,
    sid_parameter_pr,
    sid_parameter_rq,
    subfunction_rq,
    texttable_int_str_dop,
    coded_const_int_parameter,
)


def add_motor_self_test_service(dlr: DiagLayerRaw):
    """
    Add a vendor-specific RoutineControl service that the Phase 5 bench
    can surface as `/operations/motor_self_test`.

    Request layout:
    - 31 01 52 10 01  => quick mode
    - 31 01 52 10 02  => full mode

    Positive response layout:
    - 71 01 52 10 00  => passed
    - 71 01 52 10 01  => failed
    """

    mode_dop = texttable_int_str_dop(
        dlr,
        "MotorSelfTestMode",
        [
            (1, "quick"),
            (2, "full"),
        ],
    )
    result_dop = texttable_int_str_dop(
        dlr,
        "MotorSelfTestResult",
        [
            (0, "passed"),
            (1, "failed"),
        ],
    )
    dlr.diag_data_dictionary_spec.data_object_props.append(mode_dop)
    dlr.diag_data_dictionary_spec.data_object_props.append(result_dop)

    name = "motor_self_test"
    description = "Motor Self Test"

    request = Request(
        odx_id=derived_id(dlr, f"RQ.RQ_{name}"),
        short_name=f"RQ_{name}",
        parameters=NamedItemList(
            [
                sid_parameter_rq(0x31),
                subfunction_rq(0x01, "RoutineControlType"),
                coded_const_int_parameter(
                    short_name="RoutineId",
                    semantic="DATA",
                    byte_position=2,
                    coded_value_raw=str(0x5210),
                    bit_length=16,
                ),
                ValueParameter(
                    short_name="mode",
                    semantic="DATA",
                    byte_position=4,
                    dop_ref=ref(mode_dop.odx_id),
                ),
            ]
        ),
    )
    dlr.requests.append(request)

    response = Response(
        response_type=ResponseType.POSITIVE,
        odx_id=derived_id(dlr, f"PR.PR_{name}"),
        short_name=f"PR_{name}",
        parameters=NamedItemList(
            [
                sid_parameter_pr(0x31 + 0x40),
                matching_request_parameter_subfunction("RoutineControlType"),
                matching_request_parameter(
                    short_name="RoutineId",
                    semantic="DATA",
                    byte_length=2,
                    byte_position=2,
                    request_byte_position=2,
                ),
                ValueParameter(
                    short_name="result",
                    semantic="DATA",
                    byte_position=4,
                    dop_ref=ref(result_dop.odx_id),
                ),
            ]
        ),
    )
    dlr.positive_responses.append(response)

    dlr.diag_comms_raw.append(
        DiagService(
            odx_id=derived_id(dlr, f"DC.{name}"),
            short_name=name,
            long_name=description,
            functional_class_refs=[functional_class_ref(dlr, "Routine")],
            request_ref=ref(request),
            pos_response_refs=[ref(response)],
        )
    )
