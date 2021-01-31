# -*- coding: utf-8 -*-

from os import makedirs
from os.path import isfile, split
from lib.io import write_file
from lib.sutil import isNull

#---------------------------------------
# 설명 : 로그 기록.
# strFilePath   : 기록할 파일 경로.
# strDesc       : 기록할 내용
#---------------------------------------
def writeLog(strFilePath, strDesc = ''):

    if  isNull(strFilePath) is True:
        return

    strDesc = procDesc(strDesc)
        
    if  isfile(strFilePath) is True:
        write_file(strFilePath, strDesc, 'a')

    else:
        tupPath = split(strFilePath)
        try:
        #-------------------------------------
        # 디렉터리가 이미 존재한다면? except
            makedirs(tupPath[0])
        except OSError:
            pass
        
        bResult = write_file(strFilePath, strDesc)


#---------------------------------------
# 설명 : [xx월/xx일] 로그내용 으로 편집
#---------------------------------------
# writeLog 내부적으루 사용함.
def procDesc(strDesc):
    from time import time
    from lib.timeutil import frmTime_A
    
    return frmTime_A(time()) +' '+ strDesc + "\n"
