# -*- coding: utf-8 -*-

#---------------------------------------
# 설명 : 파일을 읽음
#---------------------------------------
def read_file(strFileName):

    try:
        fp = open(strFileName)
    except IOError:
        return None 
    else:
        strDesc = ''
        strLine = fp.readline()

        while strLine:
            strDesc += line + "\r"
            strLine = f.readline()

        fp.close()
        return strDesc

#---------------------------------------
# 설명 : 파일에 기록.
# strFileName   : 파일 경로
# strDesc       : 기록할 내용
# strWriteMode  : 어떤 모드로 기록할까?
#---------------------------------------
def write_file(strFileName, strDesc = '', strWriteMode = 'w'):
    
    try:
        fp = open(strFileName, strWriteMode)
    except IOError:
        return False
    else:
        fp.write(strDesc)
        fp.close()
        return True
        

def cat(ob, filename):
    try:
        f = open(filename, 'U')
    except IOError:
        ob.sendLine('[''filename' '] 열 수 없습니다.')
    
    else:
        ob.sendLine(f.read().replace('\n', '\r\n'))
        f.close()
        #ob.sendLine(f.read())
        #f.close()    

