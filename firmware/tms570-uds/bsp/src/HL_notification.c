/** @file HL_notification.c 
*   @brief User Notification Definition File
*   @date 20.May.2014
*   @version 04.00.00
*
*   This file  defines  empty  notification  routines to avoid
*   linker errors, Driver expects user to define the notification. 
*   The user needs to either remove this file and use their custom 
*   notification function or place their code sequence in this file 
*   between the provided USER CODE BEGIN and USER CODE END.
*
*/

/* Include Files */

#include "HL_esm.h"
#include "HL_adc.h"
#include "HL_can.h"
#include "HL_gio.h"
#include "HL_mibspi.h"
#include "HL_sci.h"
#include "HL_het.h"
#include "HL_rti.h"
#include "HL_epc.h"

/* USER CODE BEGIN (0) */
extern unsigned char receive_command[12];
extern unsigned int Task_Number;
extern unsigned int SubTask_Number;
extern unsigned int task_data;

void EMACCore0TxIsr(void);
void EMACCore0RxIsr(void);
/* USER CODE END */
#pragma WEAK(esmGroup1Notification)
void esmGroup1Notification(esmBASE_t *esm, uint32 channel)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (1) */
/* USER CODE END */
}

/* USER CODE BEGIN (2) */
/* USER CODE END */
#pragma WEAK(esmGroup2Notification)
void esmGroup2Notification(esmBASE_t *esm, uint32 channel)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (3) */
/* USER CODE END */
}

/* USER CODE BEGIN (4) */
/* USER CODE END */
#pragma WEAK(esmGroup3Notification)
void esmGroup3Notification(esmBASE_t *esm, uint32 channel)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (5) */
/* USER CODE END */
    for(;;)
    { 
    }/* Wait */  
/* USER CODE BEGIN (6) */
/* USER CODE END */
}

/* USER CODE BEGIN (7) */
/* USER CODE END */
#pragma WEAK(memoryPort0TestFailNotification)
void memoryPort0TestFailNotification(uint32 groupSelect, uint32 dataSelect, uint32 address, uint32 data)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (8) */
/* USER CODE END */
}

/* USER CODE BEGIN (9) */
/* USER CODE END */
#pragma WEAK(memoryPort1TestFailNotification)
void memoryPort1TestFailNotification(uint32 groupSelect, uint32 dataSelect, uint32 address, uint32 data)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (10) */
/* USER CODE END */
}

/* USER CODE BEGIN (11) */
/* USER CODE END */
#pragma WEAK(rtiNotification)
void rtiNotification(rtiBASE_t *rti, uint32 notification)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (12) */
/* USER CODE END */
}

/* USER CODE BEGIN (13) */
/* USER CODE END */
#pragma WEAK(adcNotification)
void adcNotification(adcBASE_t *adc, uint32 group)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (14) */
/* USER CODE END */
}

/* USER CODE BEGIN (15) */
/* USER CODE END */
#pragma WEAK(canErrorNotification)
void canErrorNotification(canBASE_t *node, uint32 notification)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (16) */
/* USER CODE END */
}

#pragma WEAK(canStatusChangeNotification)
void canStatusChangeNotification(canBASE_t *node, uint32 notification)  
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (17) */
/* USER CODE END */
}

#pragma WEAK(canMessageNotification)
void canMessageNotification(canBASE_t *node, uint32 messageBox)  
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (18) */
/* USER CODE END */
}

/* USER CODE BEGIN (19) */
/* USER CODE END */
#pragma WEAK(gioNotification)
void gioNotification(gioPORT_t *port, uint32 bit)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (22) */
/* USER CODE END */
}

/* USER CODE BEGIN (23) */
/* USER CODE END */
#pragma WEAK(mibspiNotification)
void mibspiNotification(mibspiBASE_t *mibspi, uint32 flags)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (28) */
/* USER CODE END */
}

/* USER CODE BEGIN (29) */
/* USER CODE END */
#pragma WEAK(mibspiGroupNotification)
void mibspiGroupNotification(mibspiBASE_t *mibspi, uint32 group)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (30) */
/* USER CODE END */
}
/* USER CODE BEGIN (31) */
/* USER CODE END */

#pragma WEAK(sciNotification)
void sciNotification(sciBASE_t *sci, uint32 flags)     
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (32) */
    /** Check for the Valid Command
     *  * - Starter
     *  ! - End of Command
     */
    if(receive_command[0] == '*' && receive_command[11] == '!')
    {
        /** - 4th and 5th byte received are Task Number,
         * combine them to form one decimal Number */
        Task_Number =  (unsigned int)(((receive_command[3] - 48) * 10) + (receive_command[4] - 48));

        /** - 6th and 7th byte received are Sub Task Number,
         * combine them to form one decimal Number */
        SubTask_Number =  (unsigned int)(((receive_command[5] - 48) * 10) + (receive_command[6] - 48));

        /** - 7th and 8th byte recieved ar data which can be used by demo
         * combine then to form a decimal number*/
        task_data      =  (unsigned int)(((receive_command[7] - 48) * 10) + (receive_command[8] - 48));
        task_data      =  (task_data<<8)|((unsigned int)(((receive_command[9] - 48) * 10) + (receive_command[10] - 48)));

        /** - Get ready to receive the next Command */
        sciReceive  (sciREG1, 12, receive_command);

        /** - Acknowledge once the Valid Command is received */
        sciSend     (sciREG1, 8, (unsigned char *) "*VALID#!");
    }
    else
    {
        /** - Get ready to receive the next Command */
        sciReceive  (sciREG1, 12, receive_command);
        /** - Acknowledge once the InValid Command is received */
        sciSend     (sciREG1, 8, (unsigned char *) "WHO R U?");
    }
/* USER CODE END */
}

/* USER CODE BEGIN (33) */
/* USER CODE END */

#pragma WEAK(pwmNotification)
void pwmNotification(hetBASE_t * hetREG,uint32 pwm, uint32 notification)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (38) */
/* USER CODE END */
}

/* USER CODE BEGIN (39) */
/* USER CODE END */
#pragma WEAK(edgeNotification)
void edgeNotification(hetBASE_t * hetREG,uint32 edge)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (40) */
/* USER CODE END */
}

/* USER CODE BEGIN (41) */
/* USER CODE END */
#pragma WEAK(hetNotification)
void hetNotification(hetBASE_t *het, uint32 offset)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (42) */
/* USER CODE END */
}

/* USER CODE BEGIN (43) */
/* USER CODE END */


/* USER CODE BEGIN (46) */
/* USER CODE END */


/* USER CODE BEGIN (50) */
/* USER CODE END */


/* USER CODE BEGIN (53) */
/* USER CODE END */


/* USER CODE BEGIN (56) */
/* USER CODE END */

#pragma WEAK(epcCAMOverflowNotification)
void epcCAMOverflowNotification(void)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (57) */
/* USER CODE END */
}
#pragma WEAK(epcFIFOOverflowNotification)
void epcFIFOOverflowNotification(uint32 epcFIFOStatus)
{
/*  enter user code between the USER CODE BEGIN and USER CODE END. */
/* USER CODE BEGIN (58) */
/* USER CODE END */
}

/* USER CODE BEGIN (59) */
/* USER CODE END */
